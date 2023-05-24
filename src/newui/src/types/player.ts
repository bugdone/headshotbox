import { RANKS } from 'src/constants/ranks';

export type SteamInfoResponse = {
  avatar: string;
  avatarfull: string;
  personaname: string;
  numberOfVacBans: number;
  daysSinceLastBan: number;
  numberOfGameBans: number;
  steamid: number;
  timestamp: number;
};

export type PlayerResponse = {
  steamid: string;
  demos: number;
  lastTimestamp: number;
  lastRank: keyof typeof RANKS;
  name: string;
  steamInfo: SteamInfoResponse;
};

export type PlayerApiResponse = {
  playerCount: number;
  players: PlayerResponse[];
};
