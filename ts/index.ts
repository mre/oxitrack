export async function on_filter_update(base_url: string, filter: string) {
  const table_body = await fetch(base_url + "/visits-table-body/" + filter).then((response) => {
    return response.text();
  });

  const table_body_element = document.getElementById("visits_table_body")!;
  table_body_element.innerHTML = table_body;
}
