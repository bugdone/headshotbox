import { Component, OnInit } from '@angular/core';
import { ApiService, Player } from '../api.service';

@Component({
  selector: 'app-player-list',
  templateUrl: './player-list.component.html',
  styleUrls: ['./player-list.component.css']
})
export class PlayerListComponent implements OnInit {
  folder: string = null;
  folders: string[] = [];

  players: Player[];
  playerCount: number = 0;
  currentPage: number = 1;
  playersPerPage: number = 20;

  constructor(private api: ApiService) { }

  ngOnInit() {
    this.refresh();
  }

  /** Refresh player list after folder or page changed */
  refresh(): void {
    this.api.getPlayers(this.folder, (this.currentPage - 1) * this.playersPerPage, this.playersPerPage)
      .then(data => {
        this.players = data.players;
        this.playerCount = data.player_count;
        // TODO: sorting in the backend?
        // TODO: pagination
        let missing_steam_info = this.players.filter(p => !p.steam_info).map(p => p.steamid);
        if (missing_steam_info.length) {
          return this.api.getSteamInfo(missing_steam_info);
        }
      }).then(steaminfo => {
        if (steaminfo) {
          for (let player of this.players) {
            if (steaminfo[player.steamid]) {
              player.steam_info = steaminfo[player.steamid];
            }
          }
        }
      });
    this.api.getFolders().then(data => this.folders = data);
  }

  sortBy(column: string): void {
    this.players.sort((a: Player, b: Player) => b[column] - a[column]);
  }

  setFolder(folder): void {
    this.folder = folder;
    this.refresh();
  }
}
