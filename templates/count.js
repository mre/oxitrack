async function count() {
  // Prevent running on testing instances like localhost.
  if (window.location.protocol !== "https:") {
    return;
  }

  // Register and get an ID.
  const registration_response = await fetch("{{ base_url }}/register?path=" + window.location.pathname);
  const registration_id = await registration_response.json();

  // Sleep the required amount before being able to call `/post-sleep`.
  await new Promise(resolve => setTimeout(resolve, parseInt("{{ sleep_secs }}") * 1000));

  // Call `/post-sleep` for the visit to be counted.
  await fetch("{{ base_url }}/post-sleep/" + registration_id, {
    referrer: document.referrer,
  });
}

count();
