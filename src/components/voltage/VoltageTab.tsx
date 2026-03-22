import { useMemo } from 'react';
import { useAppContext } from '../../App';
import VoltageRow from './VoltageRow';
import Vcom1Complex from './Vcom1Complex';
import {
  VOLTAGE_REGISTERS,
  DEFAULT_VCOM_MAX_REG_VALUE,
  DEFAULT_VCOM_MIN_REG_VALUE,
  generateHavddOptions,
  generateVcom2dacOptions,
  REG_HAVDD, REG_VCOM2DAC, REG_VCOM_MAX, REG_VCOM_MIN,
} from '../../lib/register-map';
import type { VoltageOption } from '../../lib/register-map';
import { decodeVcomLimit } from '../../lib/voltage-calc';

function VoltageTab() {
  const ctx = useAppContext();
  const avdd = ctx.getAvddVoltage();
  const vcomMinRaw = ctx.getDacValue(REG_VCOM_MIN) ?? DEFAULT_VCOM_MIN_REG_VALUE;
  const vcomMaxRaw = ctx.getDacValue(REG_VCOM_MAX) ?? DEFAULT_VCOM_MAX_REG_VALUE;
  const vcomMinV = decodeVcomLimit(vcomMinRaw, avdd);
  const vcomMaxV = decodeVcomLimit(vcomMaxRaw, avdd);

  // Dynamic options that depend on current AVDD
  const havddOptions = useMemo(() => generateHavddOptions(avdd), [avdd]);
  const vcom2dacOptions = useMemo(
    () => generateVcom2dacOptions(vcomMinV, vcomMaxV),
    [vcomMinV, vcomMaxV],
  );

  /** Get options for a voltage register, static or AVDD-dependent. */
  const getOptions = (reg: typeof VOLTAGE_REGISTERS[number]): VoltageOption[] => {
    if (reg.staticOptions) return reg.staticOptions;
    if (reg.address === REG_HAVDD) return havddOptions;
    if (reg.address === REG_VCOM2DAC) return vcom2dacOptions;
    return [];
  };

  return (
    <div className="space-y-4">
      <h2 className="text-lg font-semibold text-gray-200">Voltage Settings</h2>

      {/* Simple voltage rows */}
      <div className="space-y-1">
        {VOLTAGE_REGISTERS.map((reg) => (
          <VoltageRow key={reg.address} reg={reg} options={getOptions(reg)} />
        ))}
      </div>

      {/* VCOM1 Complex Component */}
      <Vcom1Complex />
    </div>
  );
}

export default VoltageTab;
