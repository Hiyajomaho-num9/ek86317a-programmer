import { useMemo } from 'react';
import { useLegacyFields } from '../../context/legacy';
import { useRegisterValue, useRegisterValues } from '../../context/RegistersContext';
import VoltageRow from './VoltageRow';
import Vcom1Complex from './Vcom1Complex';
import {
  REG_HAVDD,
  REG_VCOM2DAC,
  REG_VCOM_MAX,
  REG_VCOM_MIN,
  generateHavddOptions,
  generateVcom2dacOptions,
  getDefaultRegisterValue,
  getVoltageRegisters,
} from '../../lib/register-map';
import type { VoltageOption, VoltageRegisterDef } from '../../lib/register-map';
import { decodeVcomMaxLimit, decodeVcomMinLimit } from '../../lib/voltage-calc';

function VoltageTab() {
  const ctx = useLegacyFields();
  const avdd = ctx.getAvddVoltage();
  const dacRegisters = useRegisterValues();
  const vcomMin = useRegisterValue(REG_VCOM_MIN);
  const vcomMax = useRegisterValue(REG_VCOM_MAX);
  const voltageRegisters = useMemo(
    () => getVoltageRegisters(ctx.chipModel, dacRegisters),
    [ctx.chipModel, dacRegisters],
  );

  const vcomMinRaw = vcomMin ?? getDefaultRegisterValue(ctx.chipModel, REG_VCOM_MIN);
  const vcomMaxRaw = vcomMax ?? getDefaultRegisterValue(ctx.chipModel, REG_VCOM_MAX);
  const vcomMinV = decodeVcomMinLimit(vcomMinRaw, avdd);
  const vcomMaxV = decodeVcomMaxLimit(vcomMaxRaw, avdd);

  const havddOptions = useMemo(() => generateHavddOptions(avdd), [avdd]);
  const vcom2dacOptions = useMemo(
    () => generateVcom2dacOptions(vcomMinV, vcomMaxV),
    [vcomMinV, vcomMaxV],
  );

  const getOptions = (reg: VoltageRegisterDef): VoltageOption[] => {
    if (reg.staticOptions != null) return reg.staticOptions;
    if (reg.address === REG_HAVDD) return havddOptions;
    if (reg.address === REG_VCOM2DAC) return vcom2dacOptions;
    return [];
  };

  return (
    <div className="space-y-4">
      <h2 className="text-lg font-semibold text-gray-200">Voltage Settings</h2>
      <div className="space-y-1">
        {voltageRegisters.map((reg) => (
          <VoltageRow key={reg.address} reg={reg} options={getOptions(reg)} />
        ))}
      </div>
      <Vcom1Complex />
    </div>
  );
}

export default VoltageTab;
