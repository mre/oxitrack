// OxiTrack: Self-hosted, simple and privacy respecting website traffic tracker
// AGPLv3: https://codeberg.org/mo8it/oxitrack
//
// This script reports your page visit, how long you spent on the page and which referrer you possibly had.
// It doesn't collect any information that can be used to identify you.
// No IP logging, no cookies, no User-Agent, no fingerprinting!

async function count() {
  // Prevent running on testing instances like localhost
  if (window.location.protocol !== "https:") {
    return;
  }

  const fetchOptions = {
    referrer: "",
    priority: "low",
  };

  // Register, get a temporary ID and start the time measurement
  const registrationResp = await fetch(
    "{{ base_url }}/register?path=" +
      encodeURIComponent(window.location.pathname),
    fetchOptions,
  );
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

  // Sleep the required minimum amount of time before calling `/post-sleep`
  const minDelayMs = 1000 * parseInt("{{ min_delay_secs }}");
  do {
    await new Promise((r) => setTimeout(r, minDelayMs));

    const newStartTime = new Date();
    timeOnPageMs += newStartTime.getTime() - startTime.getTime();
    startTime = newStartTime;
  } while (timeOnPageMs < minDelayMs);

  let queryParams = "";
  // Extract the referrer origin respecting the `no-referrer` policy
  try {
    const referrer = new URL(document.referrer);
    if (
      referrer.protocol === "https:" &&
      referrer.origin !== window.location.origin
    ) {
      queryParams = "?referrer_origin=" + encodeURIComponent(referrer.origin);
    }
  } catch {}

  // Call `/post-sleep` for the visit to be counted
  await fetch(
    "{{ base_url }}/post-sleep/" + visitorId + queryParams,
    fetchOptions,
  );

  // On leaving the page, call `/page-left` to report the total spent time in seconds
  window.addEventListener("beforeunload", async () => {
    timeOnPageMs += new Date().getTime() - startTime.getTime();
    const timeOnPageS = Math.round(0.001 * timeOnPageMs);
    await fetch(
      "{{ base_url }}/page-left/" + visitorId + "/" + timeOnPageS,
      fetchOptions,
    );
  });
}

count();
