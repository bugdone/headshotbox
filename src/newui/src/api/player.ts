import type { ApiRequestParams, StatsRequestParams } from '@/types/api';
import type { PlayerApiResponse, PlayerInfoResponse, PlayerStats, PlayerTeamMate } from '@/types/player';

import { api } from 'boot/axios';

export const PlayerApi = {
  list: async (params: ApiRequestParams = { limit: 50, offset: 0 }): Promise<PlayerApiResponse> => {
    const { data } = await api.get<PlayerApiResponse>('players', {
      params,
    });

    return data;
  },

  getPlayerInfo: async (steamId: string): Promise<PlayerInfoResponse> => {
    const { data } = await api.get<PlayerInfoResponse>('steamids/info', { params: { steamids: steamId } });

    return data;
  },

  stats: async (steamId: string, params: StatsRequestParams): Promise<PlayerStats> => {
    const { data } = await api.get<PlayerStats>(`player/${steamId}/stats`, { params });

    return data;
  },

  maps: async (steamId: string): Promise<string[]> => {
    const { data } = await api.get<string[]>(`player/${steamId}/maps`);

    return data;
  },

  teamMates: async (steamId: string): Promise<PlayerTeamMate[]> => {
    const { data } = await api.get<PlayerTeamMate[]>(`player/${steamId}/teammates`);

    return data;
  },
};
