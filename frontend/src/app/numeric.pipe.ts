import { Pipe, PipeTransform } from '@angular/core';
import { DecimalPipe } from '@angular/common';

@Pipe({ name: 'numeric' })
export class NumericPipe implements PipeTransform {
  constructor(private dp: DecimalPipe) { }

  transform(value: any, decimal_places: number): any {
    if (value !== undefined && !isNaN(value)) {
      return this.dp.transform(value, `.${decimal_places}-${decimal_places}`);
    }
  }
}
