import { Chart } from "chart.js/auto";

export async function render_bar_chart(data: Array<{ x: string, y: number }>, max_count: number, date_trunc: string) {
  if (data.length == 0) {
    return;
  }

  new Chart(
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
          text: "visits/" + date_trunc
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
          max: max_count,
          ticks: {
            precision: 0,
            minRotation: 0,
            maxRotation: 0
          }
        }
      }
    }
  });
}
