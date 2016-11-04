import { Component, OnInit, Input, Output, EventEmitter } from '@angular/core';

@Component({
  selector: 'app-dropdown',
  templateUrl: './dropdown.component.html'
})
export class DropdownComponent {
  // Label on the button before the currently selected value
  @Input() label: string;
  // How to display null values
  @Input() nullDisplay: string = "null";
  // List of items in the dropdown
  @Input() items: any[];
  // currently selected value
  @Input() value: any;
  @Output() valueChange = new EventEmitter<any>();

  setValue(item: any) {
    if (this.value !== item) {
      this.value = item;
      this.valueChange.emit(item);
    }
  }
}
