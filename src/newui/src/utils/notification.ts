import type { Alert } from '@/types/alert';
import { Notify } from 'quasar';

const messageFn = (type: Alert['type'], message: string, timeout = 3000) => {
  let color = 'primary';
  switch (type) {
    case 'success':
      color = 'green';
      break;
    case 'warning':
      color = 'orange';
      break;
    case 'error':
      color = 'red';
      break;
  }

  Notify.create({
    message,
    color,
    timeout,
    textColor: 'white',
    progress: true,
    position: 'top',
  });
};

export default {
  error: (message: string, timeout = 3000) => messageFn('error', message, timeout),

  warning: (message: string, timeout = 3000) => messageFn('warning', message, timeout),

  success: (message: string, timeout = 3000) => messageFn('success', message, timeout),
};
