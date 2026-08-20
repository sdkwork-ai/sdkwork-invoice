import type { Invoice } from './invoice';
import type { PageInfo } from './page-info';

export interface InvoicePageResponse {
  code: 0;
  data: { items: Invoice[]; pageInfo: PageInfo; };
  traceId: string;
}
