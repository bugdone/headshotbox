import { Loading } from 'quasar';

export default {
  simple: () => Loading.show(),

  hide: () => Loading.hide(),

  withMessage: (message: string) =>
    Loading.show({
      message: message + ' ...',
    }),

  isLoading: (): boolean => Loading.isActive,
};
