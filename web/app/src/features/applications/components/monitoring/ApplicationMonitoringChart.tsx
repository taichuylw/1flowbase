import { useEffect, useRef } from 'react';

import { BarChart, LineChart, PieChart } from 'echarts/charts';
import {
  GridComponent,
  LegendComponent,
  TooltipComponent
} from 'echarts/components';
import * as echarts from 'echarts/core';
import { CanvasRenderer } from 'echarts/renderers';

echarts.use([
  BarChart,
  LineChart,
  PieChart,
  GridComponent,
  LegendComponent,
  TooltipComponent,
  CanvasRenderer
]);

export function ApplicationMonitoringChart({
  ariaLabel,
  option
}: {
  ariaLabel: string;
  option: echarts.EChartsCoreOption;
}) {
  const chartRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!chartRef.current) {
      return;
    }

    const chart = echarts.init(chartRef.current);
    chart.setOption(option);

    const resizeObserver =
      typeof ResizeObserver === 'undefined'
        ? null
        : new ResizeObserver(() => {
            chart.resize();
          });
    resizeObserver?.observe(chartRef.current);

    return () => {
      resizeObserver?.disconnect();
      chart.dispose();
    };
  }, [option]);

  return (
    <div
      ref={chartRef}
      aria-label={ariaLabel}
      className="application-monitoring-chart"
      role="img"
    />
  );
}
