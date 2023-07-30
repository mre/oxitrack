"use strict";

// #######################################################################
// Enter the URL to your OxiTraffic instance.
const oxitraffic_base_url = "https://YOUR_OXITRAFFIC_URL";
// Enter the minimum delay in seconds before the visit can be counted.
// Recommendation: OxiTraffic's configuration option `min_delay_secs` + 1.
// Using 19 + 1 = 20 for the default `min_delay_secs = 19`.
const sleep_in_seconds = 20;
// #######################################################################

// Register and get an ID.
const registration_response = await fetch(oxitraffic_base_url + "/register?path=" + window.location.pathname);
const registration_id = await registration_response.json();

// Sleep the required amount before being able to call `/post-sleep`.
await new Promise(resolve => setTimeout(() => resolve(), sleep_in_seconds * 1000));

// Call `/post-sleep` for the visit to be counted.
await fetch(oxitraffic_base_url + "/post-sleep/" + registration_id);

console.log("Your visit has been successfully counted by OxiTraffic :)");
