export const Format = {
  number: (value: number) =>
    new Intl.NumberFormat(window.navigator.language, {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(value),
};
