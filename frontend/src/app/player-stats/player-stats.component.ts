import { Component, OnInit, OnChanges, Input } from '@angular/core';
import { ApiService, SteamInfo, DemoFilters } from '../api.service';

@Component({
  selector: 'app-player-stats',
  templateUrl: './player-stats.component.html',
  styleUrls: ['./player-stats.component.css']
})
export class PlayerStatsComponent implements OnInit, OnChanges {
  @Input() steamid: string;
  @Input() filters: DemoFilters;

  steam_info: SteamInfo;
  stats = { rounds_with_kills: [] };

  constructor(private api: ApiService) { }

  ngOnInit() {
    // TODO last_rank?
    // TODO use cached player?
    this.api.getSteamInfo([this.steamid]).then(steam_info => {
      this.steam_info = steam_info[this.steamid];
    });
  }

  ngOnChanges() {
    this.refreshStats();
  }

  refreshStats() {
    this.api.getPlayerStats(this.steamid, this.filters).then(data => {
      this.stats = data;
    });
  }
}
