import { useMemo } from 'react';
import { useAppContext } from '../../App';
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
import { decodeVcomLimit } from '../../lib/voltage-calc';

function VoltageTab() {
  const ctx = useAppContext();
  const avdd = ctx.getAvddVoltage();
  const voltageRegisters = useMemo(
    () => getVoltageRegisters(ctx.chipModel, ctx.dacRegisters),
    [ctx.chipModel, ctx.dacRegisters],
  );

  const vcomMinRaw = ctx.getDacValue(REG_VCOM_MIN) ?? getDefaultRegisterValue(ctx.chipModel, REG_VCOM_MIN);
  const vcomMaxRaw = ctx.getDacValue(REG_VCOM_MAX) ?? getDefaultRegisterValue(ctx.chipModel, REG_VCOM_MAX);
  const vcomMinV = decodeVcomLimit(vcomMinRaw, avdd);
  const vcomMaxV = decodeVcomLimit(vcomMaxRaw, avdd);

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
