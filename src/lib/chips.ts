export type ChipModel = 'ek86317a' | 'iml8947k' | 'lp6281';

export interface ChipOption {
  value: ChipModel;
  label: string;
}

export interface ChipCapabilities {
  hasVcomSlave: boolean;
  supportsFault: boolean;
  supportsVcom2dac: boolean;
  supportsMntMode: boolean;
  primaryVcomName: string;
  secondaryVcomName: string;
  exportPrefix: string;
}

export const CHIP_OPTIONS: ChipOption[] = [
  { value: 'ek86317a', label: 'EK86317A' },
  { value: 'iml8947k', label: 'iML8947K' },
  { value: 'lp6281', label: 'LP6281' },
];

const CHIP_CAPABILITIES: Record<ChipModel, ChipCapabilities> = {
  ek86317a: {
    hasVcomSlave: true,
    supportsFault: true,
    supportsVcom2dac: true,
    supportsMntMode: false,
    primaryVcomName: 'VCOM1_NT',
    secondaryVcomName: 'VCOM1_HT',
    exportPrefix: 'ek86317a',
  },
  iml8947k: {
    hasVcomSlave: true,
    supportsFault: true,
    supportsVcom2dac: true,
    supportsMntMode: true,
    primaryVcomName: 'VCOM1_NT',
    secondaryVcomName: 'VCOM1_HT',
    exportPrefix: 'iml8947k',
  },
  lp6281: {
    hasVcomSlave: false,
    supportsFault: false,
    supportsVcom2dac: false,
    supportsMntMode: false,
    primaryVcomName: 'VCOM_NT',
    secondaryVcomName: 'VCOM_HT',
    exportPrefix: 'lp6281',
  },
};

export function getChipDisplayName(model: ChipModel): string {
  return CHIP_OPTIONS.find((option) => option.value === model)?.label ?? model;
}

export function getChipCapabilities(model: ChipModel): ChipCapabilities {
  return CHIP_CAPABILITIES[model];
}

export function getChipStoragePrefix(model: ChipModel): string {
  return CHIP_CAPABILITIES[model].exportPrefix;
}
