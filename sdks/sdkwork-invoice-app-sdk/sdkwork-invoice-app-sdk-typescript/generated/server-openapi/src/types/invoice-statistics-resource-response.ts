import type { InvoiceStatistics } from './invoice-statistics';

export interface InvoiceStatisticsResourceResponse {
  code: 0;
  data: { item: InvoiceStatistics; };
  traceId: string;
}
