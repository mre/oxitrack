import { Chart } from "chart.js/auto";

export async function render_bar_chart(base_url: string, path: string) {
  const query_params = "?path=" + encodeURIComponent(path);
  const chart_data_url = base_url + "/chart-data/";

  async function chart_data(filter: string): Promise<Array<{ x: string, y: number }>> {
    return fetch(chart_data_url + filter + query_params).then((response) => {
      return response.json();
    });
  }

  const data = await chart_data("last-60-days");

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

  for (const filter of ["last-60-days", "all-time"]) {
    const btn = document.getElementById(filter) as HTMLInputElement;

    btn.addEventListener("change", async () => {
      const data = await chart_data(filter);

      chart.data.datasets[0]!.data = data;
      chart.update();
    })
  }
}
