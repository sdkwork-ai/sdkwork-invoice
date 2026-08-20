import type { InvoiceItem } from './invoice-item';
import type { PageInfo } from './page-info';

export interface InvoiceItemPageResponse {
  code: 0;
  data: { items: InvoiceItem[]; pageInfo: PageInfo; };
  traceId: string;
}
