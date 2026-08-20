import type { Invoice } from './invoice';

export interface InvoiceResourceResponse {
  code: 0;
  data: { item: Invoice; };
  traceId: string;
}
