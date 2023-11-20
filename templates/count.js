async function count() {
  // Prevent running on testing instances like localhost
  if (window.location.protocol !== "https:") {
    return;
  }

  // Register, get an ID and start time measurement
  const registrationResp = await fetch("{{ base_url }}/register?path=" + encodeURIComponent(window.location.pathname));
  let startTime = new Date();
  let timeOnPageMs = 0;
  const visitorId = await registrationResp.json();

  // Measure time only while the page is visible
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") {
      timeOnPageMs += new Date().getTime() - startTime.getTime();
    } else {
      startTime = new Date();
    }
  });

  // Sleep the required amount before calling `/post-sleep`
  const minDelayMs = 1000 * parseInt("{{ min_delay_secs }}");
  do {
    await new Promise(r => setTimeout(r, minDelayMs));

    const newStartTime = new Date();
    timeOnPageMs += newStartTime.getTime() - startTime.getTime();
    startTime = newStartTime;
  } while (timeOnPageMs < minDelayMs);

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
