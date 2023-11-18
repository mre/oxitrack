async function count() {
  // Prevent running on testing instances like localhost
  if (window.location.protocol !== "https:") {
    return;
  }

  let startTime = new Date();
  let timeOnPageMs = 0;

  // Register and get an ID
  const registrationResp = await fetch("{{ base_url }}/register?path=" + encodeURIComponent(window.location.pathname));
  const visitorId = await registrationResp.json();

  // Measure time only while the page is visible
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") {
      timeOnPageMs += new Date().getTime() - startTime.getTime();
    } else {
      startTime = new Date();
    }
  });

  // Sleep the required amount before being able to call `/post-sleep`
  await new Promise(r => setTimeout(r, 1000 * parseInt("{{ sleep_secs }}")));

  // Prepare query parameters
  let queryParams = "";
  if (document.referrer.length > 0) {
    try {
      const referrer = new URL(document.referrer);
      if (referrer.protocol === "https:" && referrer.origin !== window.location.origin) {
        queryParams = "?referrer_origin=" + encodeURIComponent(referrer.origin);
      }
    } catch (e) {
      console.log(e);
    }
  }

  // Call `/post-sleep` for the visit to be counted
  await fetch("{{ base_url }}/post-sleep/" + visitorId + queryParams);

  // Call `/page-left` to report the total spent time
  window.addEventListener("beforeunload", async () => {
    timeOnPageMs += new Date().getTime() - startTime.getTime();
    const timeOnPageS = Math.round(0.001 * timeOnPageMs);
    await fetch("{{ base_url }}/page-left/" + visitorId + "/" + timeOnPageS);
  });
}

count();
