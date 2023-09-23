import { Chart } from "chart.js/auto";
import "chartjs-adapter-date-fns";

async function render_bar_chart(history: Array<number>) {
  const data = [];

  const now = new Date();
  const now_day = now.getDate();
  const now_month = now.getMonth();
  const now_year = now.getFullYear();

  const first_timestamp = history[0]!;
  let iter_date = new Date(first_timestamp);
  let iter_day = iter_date.getDate();
  let iter_month = iter_date.getMonth();
  let iter_year = iter_date.getFullYear();
  let count = 1;
  let max_count = count;

  // Skip first element.
  let skip = true;
  for (let timestamp of history) {
    if (skip) {
      skip = false;
      continue;
    }

    const date = new Date(timestamp);
    const day = date.getDate();
    const month = date.getMonth();
    const year = date.getFullYear();

    if (day == iter_day && month == iter_month && year == iter_year) {
      count += 1;
    } else {
      do {
        data.push({ x: `${iter_day}.${iter_month}.${iter_year}`, y: count });
        if (count > max_count) {
          max_count = count;
        }

        iter_date.setDate(iter_day + 1);
        iter_day = iter_date.getDate();
        iter_month = iter_date.getMonth();
        iter_year = iter_date.getFullYear();
        count = 0;
      } while (day != iter_day || month != iter_month || year != iter_year);

      count = 1;
    }
  }

  while (now_day != iter_day || now_month != iter_month || now_year != iter_year) {
    data.push({ x: `${iter_day}.${iter_month}.${iter_year}`, y: count });
    if (count > max_count) {
      max_count = count;
    }

    iter_date.setDate(iter_day + 1);
    iter_day = iter_date.getDate();
    iter_month = iter_date.getMonth();
    iter_year = iter_date.getFullYear();
    count = 0;
  }

  data.push({ x: `${iter_day}.${iter_month}.${iter_year}`, y: count });
  if (count > max_count) {
    max_count = count;
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
          text: 'visits/day'
        },
        legend: {
          display: false
        }
      },
      scales: {
        x: {
          type: 'category',
          ticks: {
            minRotation: 50,
            maxRotation: 50,
            includeBounds: false
          }
        },
        y: {
          type: 'linear',
          min: 0,
          max: max_count,
          ticks: {
            stepSize: 1,
            minRotation: 0,
            maxRotation: 0
          }
        }
      }
    }
  });
}

async function render_line_chart(history: Array<number>, min_chart_timestamp: number, max_chart_timestamp: number) {
  const data = history.map(function(element, index) { return { x: element, y: index + 1 }; });

  new Chart(
    document.getElementById('line_chart') as HTMLCanvasElement, {
    type: 'line',
    data: {
      datasets: [{
        label: 'Visit',
        data: data
      }]
    },
    options: {
      maintainAspectRatio: false,
      animation: false,
      parsing: false,
      plugins: {
        title: {
          display: true,
          text: 'visits over time'
        },
        legend: {
          display: false
        }
      },
      elements: {
        point: {
          radius: 2,
          hitRadius: 4
        },
        line: {
          borderWidth: 1
        }
      },
      scales: {
        x: {
          type: 'time',
          min: min_chart_timestamp,
          max: max_chart_timestamp,
          ticks: {
            minRotation: 50,
            maxRotation: 50,
            includeBounds: false
          }
        },
        y: {
          type: 'linear',
          min: 0,
          max: history.length,
          ticks: {
            stepSize: 1,
            minRotation: 0,
            maxRotation: 0
          }
        }
      }
    }
  });
}

export async function render_charts(history: Array<number>, min_chart_timestamp: number, max_chart_timestamp: number) {
  if (history.length == 0) {
    return;
  }

  render_bar_chart(history);
  render_line_chart(history, min_chart_timestamp, max_chart_timestamp);
}
