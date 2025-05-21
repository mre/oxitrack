import { Chart } from "chart.js/auto";

type StatsData = {
  chart_data: Array<{ x: string, y: number }>;
  table_body: string;
};

const colorSchemeQueryList = window.matchMedia('(prefers-color-scheme: dark)');

export async function render_bar_chart(base_url: string, path?: string) {
  const query_params = (path !== undefined) ? "?path=" + encodeURIComponent(path) : "";
  const chart_data_url = base_url + "/stats-data/";

  async function chart_data(filter: string): Promise<StatsData> {
    return fetch(chart_data_url + filter + query_params).then((response) => {
      return response.json();
    });
  }

  const checked_filter = document.querySelector("input[name='filter']:checked") as HTMLInputElement;

  const data = await chart_data(checked_filter.value);

  const table = document.getElementById("table")!;
  const chart_canvas = document.getElementById("bar_chart")!;
  const chart_canvas_table = document.querySelector("#bar_chart table")!;
  const table_body_element = document.getElementById("table_body")!;

  function update_table(data: StatsData) {
    if (data.table_body.length > 0) {
      table_body_element.innerHTML = data.table_body;
      table.hidden = false;
    } else {
      table.hidden = true;
    }
  }

  function update_canvas_table(data: StatsData) {
    document.querySelectorAll("#bar_chart tr").forEach(e => e.remove());
    for (const row of data.chart_data) {
      let th = document.createElement("th");
      th.innerText = row.x;
      let td = document.createElement("td");
      td.innerText = `${row.y}`;
      let tr = document.createElement("tr");
      tr.appendChild(th);
      tr.appendChild(td);
      chart_canvas_table.appendChild(tr);
    }
  }

  update_table(data);
  update_canvas_table(data);

  let color;
  if (colorSchemeQueryList.matches) {
    color = '#FFF';
  } else {
    color = '#000';
  }

  const chart = new Chart(
    chart_canvas as HTMLCanvasElement, {
    type: 'bar',
    data: {
      datasets: [{
        label: 'Visits',
        data: data.chart_data
      }]
    },
    options: {
      maintainAspectRatio: false,
      animation: {
        duration: 2000
      },
      color,
      plugins: {
        title: {
          display: true,
          text: "visits",
          color,
        },
        legend: {
          display: false
        }
      },
      scales: {
        x: {
          type: 'category',
          ticks: {
            minRotation: 60,
            maxRotation: 60,
            includeBounds: false,
            color,
          },
          grid: {
            display: false,
          }
        },
        y: {
          type: 'linear',
          min: 0,
          ticks: {
            precision: 0,
            minRotation: 0,
            maxRotation: 0,
            color,
          }
        }
      }
    }
  });

  for (const filter of ["last-2-days", "last-60-days", "all-time"]) {
    const btn = document.getElementById(filter) as HTMLInputElement;

    btn.addEventListener("change", async () => {
      const data = await chart_data(filter);

      chart.data.datasets[0]!.data = data.chart_data;
      chart.update();

      update_table(data);
      update_canvas_table(data);
    })
  }

  colorSchemeQueryList.addEventListener("change", () => {
    if (colorSchemeQueryList.matches) {
      chart.options.color = '#FFF';
      chart.options.plugins!.title!.color =  '#FFF';
      chart.options.scales!["x"]!.ticks!.color =  '#FFF';
      chart.options.scales!["y"]!.ticks!.color =  '#FFF';
    } else {
      chart.options.color = '#000';
      chart.options.plugins!.title!.color =  '#000';
      chart.options.scales!["x"]!.ticks!.color =  '#000';
      chart.options.scales!["y"]!.ticks!.color =  '#000';
    }

    chart.update();
  });
}

(window as any).render_bar_chart = render_bar_chart;
