import { Dictionary } from 'types/common';

export const DEMO_TYPES = {
  VALVE: 'valve',
  ESEA: 'esea',
  FACEIT: 'faceit',
  CEVO: 'cevo',
  ESPORTAL: 'esportal',
  CUSTOM: 'custom',
};

export const DEMO_TYPE_IMAGES: Dictionary = {};
DEMO_TYPE_IMAGES[DEMO_TYPES.VALVE] = 'images/demoTypes/valve.png';
DEMO_TYPE_IMAGES[DEMO_TYPES.ESEA] = 'images/demoTypes/esea.png';
DEMO_TYPE_IMAGES[DEMO_TYPES.FACEIT] = 'images/demoTypes/faceit.png';
DEMO_TYPE_IMAGES[DEMO_TYPES.ESPORTAL] = 'images/demoTypes/esportal.png';
DEMO_TYPE_IMAGES[DEMO_TYPES.CUSTOM] = 'images/demoTypes/custom.png';
