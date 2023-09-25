export async function count() {
  // Prevent running on testing instances like localhost
  if (window.location.protocol !== "https:") {
    return;
  }

  // Register and get an ID
  const registration_resp = await fetch("{{ base_url }}/register?path=" + window.location.pathname);
  const registration_id = await registration_resp.json();

  // Sleep the required amount before being able to call `/post-sleep`
  await new Promise(r => setTimeout(r, 1000 * parseInt("{{ sleep_secs }}")));

  // Call `/post-sleep` for the visit to be counted
  await fetch("{{ base_url }}/post-sleep/" + registration_id, {
    referrer: document.referrer
  });
}
