import { Component, OnInit, Input, Output, EventEmitter } from '@angular/core';

@Component({
  selector: 'app-dropdown',
  templateUrl: './dropdown.component.html'
})
export class DropdownComponent implements OnInit {
  // Label on the button before the currently selected value
  @Input() label: string;
  // How to display null values
  @Input() nullDisplay: string = 'null';
  // List of items in the dropdown
  @Input() items: any[];
  // currently selected value
  @Input() value: any;
  @Output() valueChange = new EventEmitter<any>();

  currentLabel: string;

  ngOnInit() {
    this.updateLabel();
  }

  setValue(item: any) {
    if (this.value !== item) {
      this.value = item;
      this.updateLabel();
      this.valueChange.emit(item);
    }
  }

  private updateLabel() {
    this.currentLabel = this.label.replace('$', this.value ? this.value : this.nullDisplay);
  }
}
