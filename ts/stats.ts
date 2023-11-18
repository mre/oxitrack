import { Chart } from "chart.js/auto";

type StatsData = {
  chart_data: Array<{ x: string, y: number }>;
  table_body: string;
};

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
  const table_body_element = document.getElementById("table_body")!;

  function update_table(data: StatsData) {
    if (data.table_body.length > 0) {
      table_body_element.innerHTML = data.table_body;
      table.hidden = false;
    } else {
      table.hidden = true;
    }
  }

  update_table(data);

  const chart = new Chart(
    document.getElementById('bar_chart') as HTMLCanvasElement, {
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
      plugins: {
        title: {
          display: true,
          text: "visits"
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
            includeBounds: false
          },
          grid: {
            display: false
          }
        },
        y: {
          type: 'linear',
          min: 0,
          ticks: {
            precision: 0,
            minRotation: 0,
            maxRotation: 0
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
    })
  }
}
