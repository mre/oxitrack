"use strict";

const oxitraffic_base_url = "https://YOUR_OXITRAFFIC_URL";

const registration_response = await fetch(oxitraffic_base_url + "/register?path=" + window.location.pathname);
const registration_id = await registration_response.json();
await new Promise(resolve => setTimeout(() => resolve(), 15000));
await fetch(oxitraffic_base_url + "/post-sleep/" + registration_id);
console.log("Successful OxiTraffic report :)");
