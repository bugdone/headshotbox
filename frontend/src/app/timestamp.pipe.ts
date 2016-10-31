import { Pipe, PipeTransform } from '@angular/core';

@Pipe({name: 'timestamp'})
export class TimestampPipe implements PipeTransform {
  transform(timestamp: number, format?: string): any {
    if (!timestamp) {
      return '';
    }
    let d = new Date(timestamp * 1000);
    let locale_format: any = { day: 'numeric', month: 'short' };
    if (format !== 'date') {
      locale_format.hour = '2-digit';
      locale_format.minute = '2-digit';
      locale_format.hour12 = false;
    }
    if (d.getFullYear() !== (new Date()).getFullYear()) {
      locale_format.year = 'numeric';
    }
    return d.toLocaleString(undefined, locale_format);
  }
}
