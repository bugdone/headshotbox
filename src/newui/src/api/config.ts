import type { DemoFolders } from '@/types/config';

import { api } from 'boot/axios';

export const ConfigApi = {
  folders: async (): Promise<DemoFolders> => {
    const { data } = await api.get<DemoFolders>('folders');

    return data;
  },
};
