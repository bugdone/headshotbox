import { Component, OnInit, Input, Output, EventEmitter } from '@angular/core';
import { ApiService, Teammate } from '../api.service';

@Component({
  selector: 'app-teammates-picker',
  templateUrl: './teammates-picker.component.html',
  styleUrls: ['./teammates-picker.component.css']
})
export class TeammatesPickerComponent implements OnInit {
  @Input() steamid: string;
  @Input() value: string[];
  @Output() valueChange = new EventEmitter<string[]>();
  playerTeammates: Teammate[];
  selected: Teammate[] = [];

  constructor(private api: ApiService) { }

  ngOnInit() {
    this.api.getPlayerTeammates(this.steamid).then(data => {
      this.playerTeammates = data;
      return this.api.fillSteamInfo(this.playerTeammates);
    });
  }

  add(teammate: Teammate) {
    this.selected.push(teammate);
    this.emit();
  }

  remove(teammate: Teammate) {
    let i = this.selected.indexOf(teammate);
    if (i == -1)
      return;
    this.selected.splice(i, 1);
    this.emit();
  }

  emit() {
    this.value = this.selected.map(p => p.steamid);
    this.valueChange.emit(this.value);
  }
}
