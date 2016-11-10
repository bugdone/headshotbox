import { Component, OnInit, Input, Output, EventEmitter } from '@angular/core';

@Component({
  selector: 'app-datepicker',
  templateUrl: './datepicker.component.html',
  styleUrls: ['./datepicker.component.css']
})
export class DatepickerComponent implements OnInit {
  @Input() label: string;
  @Input() minDate: Date;
  // currently selected value
  @Input() value: Date;
  @Output() valueChange = new EventEmitter<Date>();
  private isOpen: boolean;

  constructor() { }

  ngOnInit() {
  }

  setValue() {
    this.valueChange.emit(this.value);
    this.isOpen = false;
  }
}
