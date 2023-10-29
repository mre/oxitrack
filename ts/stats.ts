import { Chart } from "chart.js/auto";

interface Params {
  path?: string;
  on_filter_update?: (base_url: string, filter: string) => Promise<void>;
}

export async function render_bar_chart(base_url: string, params: Params = {}) {
  const query_params = (params.path !== undefined) ? "?path=" + encodeURIComponent(params.path) : "";
  const chart_data_url = base_url + "/chart-data/";

  async function chart_data(filter: string): Promise<Array<{ x: string, y: number }>> {
    return fetch(chart_data_url + filter + query_params).then((response) => {
      return response.json();
    });
  }

  const checked_filter = document.querySelector("input[name='filter']:checked") as HTMLInputElement;

  const data = await chart_data(checked_filter.value);

  if (params.on_filter_update !== undefined) {
    await params.on_filter_update(base_url, checked_filter.value);
  }

  const chart = new Chart(
    document.getElementById('bar_chart') as HTMLCanvasElement, {
    type: 'bar',
    data: {
      datasets: [{
        label: 'Visits',
        data: data
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

      chart.data.datasets[0]!.data = data;
      chart.update();

      if (params.on_filter_update !== undefined) {
        await params.on_filter_update(base_url, filter);
      }
    })
  }
}
