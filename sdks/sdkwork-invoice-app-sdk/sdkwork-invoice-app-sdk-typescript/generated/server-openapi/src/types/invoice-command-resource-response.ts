import type { InvoiceCommand } from './invoice-command';

export interface InvoiceCommandResourceResponse {
  code: 0;
  data: { item: InvoiceCommand; };
  traceId: string;
}
