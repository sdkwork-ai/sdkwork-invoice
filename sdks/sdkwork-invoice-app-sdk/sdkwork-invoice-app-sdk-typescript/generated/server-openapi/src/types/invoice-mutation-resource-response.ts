import type { InvoiceMutation } from './invoice-mutation';

export interface InvoiceMutationResourceResponse {
  code: 0;
  data: { item: InvoiceMutation; };
  traceId: string;
}
