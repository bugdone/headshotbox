import { Injectable } from '@angular/core';
import { Http, URLSearchParams, RequestOptionsArgs } from '@angular/http';

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
    return this.getPromise('/players', {search: params});
  }

  getSteamInfo(steamids: string[]): Promise<SteamInfoMap> {
    let params = new URLSearchParams();
    params.set('steamids', steamids.join(','));
    return this.getPromise('/steamids/info', {search: params});
  }

  getFolders(): Promise<string[]> {
    return this.getPromise('/folders');
  }

  getVersion(): Promise<{current: string, latest: string}> {
    return this.getPromise('/version');
  }

  getAuthorization(): Promise<{authorized: boolean, showLogin: boolean}> {
    return this.getPromise('/authorized');
  }

  private getPromise(path: string, options?: RequestOptionsArgs): Promise<any> {
    return this.http.get(this.serverUrl + path, options).toPromise().then(r => r.json()).catch(this.handleError);
  }

  private handleError(error: any): Promise<any> {
    return Promise.reject(error.message || error);
  }
}
