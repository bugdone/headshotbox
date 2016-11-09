import { Component, OnInit, Input, Output, EventEmitter } from '@angular/core';
import { ApiService, DemoFilters } from '../api.service';

@Component({
  selector: 'app-demo-filters',
  templateUrl: './demo-filters.component.html',
  styleUrls: ['./demo-filters.component.css']
})
export class DemoFiltersComponent implements OnInit {
  @Input() steamid: string;
  @Input() filters: DemoFilters;
  @Output() filtersChange = new EventEmitter<DemoFilters>();

  current = new DemoFilters();
  folders: string[];
  playerMaps: string[];

  constructor(private api: ApiService) { }

  ngOnInit() {
    this.api.getFolders().then(data => this.folders = data);
    this.api.getPlayerMaps(this.steamid).then(data => this.playerMaps = [null].concat(data));
  }

  setDemoType(demoType) {
    this.current.demoType = demoType;
    this.emit();
  }

  emit() {
    // Trigger change detection
    this.current = Object.assign(new DemoFilters(), this.current);
    this.filtersChange.emit(this.current);
  }
}
