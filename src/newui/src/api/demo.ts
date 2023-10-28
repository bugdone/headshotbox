import { ApiRequestParams } from '@/types/api';
import { DemoApiResponse, DemoStats } from '@/types/demo';

import { api } from 'boot/axios';

export const DemoApi = {
  list: async (steamId: string, params: ApiRequestParams = { limit: 50, offset: 0 }): Promise<DemoApiResponse> => {
    const { data } = await api.get<DemoApiResponse>(`player/${steamId}/demos`, {
      params,
    });

    return data;
  },

  details: async (demoId: number): Promise<DemoStats> => {
    const { data } = await api.get<DemoStats>(`demo/${demoId}/stats/`);

    return data;
  },
};
