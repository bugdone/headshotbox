export interface Alert {
  message: string;
  type: 'success' | 'info' | 'warning' | 'error';
  timeout?: number; // Time in milliseconds. The default of the Notify plugin is 5000.
  position?: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right' | 'top' | 'bottom' | 'left' | 'right' | 'center';
}
