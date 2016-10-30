import { Injectable } from '@angular/core';
import { Http, URLSearchParams } from '@angular/http';

import 'rxjs/add/operator/toPromise';

export class Player {
  steamid: string;
  name: string;
  demos: number;
  last_rank: number;
  last_timestamp: number;
  steam_info?: SteamInfo;
}

export class Players {
  player_count: number;
  players: Player[];
}

export class SteamInfo {
  DaysSinceLastBan: number;
  NumberOfGameBans: number;
  NumberOfVACBans: number;
  avatar: string;
  avatarfull: string;
  last_rank: number;
  personaname: string;
  timestamp: number;
}

export interface SteamInfoMap {
  [steamid: string]: SteamInfo;
}

@Injectable()
export class ApiService {
  serverUrl: string = 'http://localhost:4000/api';
  constructor(private http: Http) { }

  getPlayers(folder: string, offset: number, limit: number): Promise<Players> {
    let params = new URLSearchParams();
    params.set('offset', String(offset));
    params.set('limit', String(limit));
    if (folder !== null) {
      params.set('folder', folder);
    }
    return this.http.get(this.serverUrl + '/players', {search: params}).toPromise()
      .then(r => r.json() as Players)
      .catch(this.handleError);
  }

  getSteamInfo(steamids: string[]): Promise<SteamInfoMap> {
    let params = new URLSearchParams();
    params.set('steamids', steamids.join(','));
    return this.http.get(this.serverUrl + '/steamids/info', {search: params}).toPromise()
      .then(r => r.json() as SteamInfoMap)
      .catch(this.handleError);
  }

  getFolders(): Promise<string[]> {
    return this.http.get(this.serverUrl + '/folders').toPromise().then(r => r.json());
  }

  handleError(error: any): Promise<any> {
    return Promise.reject(error.message || error);
  }
}
