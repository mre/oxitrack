"use strict";

const oxitraffic_base_url = "https://oxitraffic.mo8it.com";

const registration_id = await fetch(oxitraffic_base_url + "/register?path=" + window.location.pathname).then((
  response,
) => response.json());
await new Promise(resolve => setTimeout(() => resolve(), 15000));
await fetch(oxitraffic_base_url + "/post-sleep/" + registration_id);
