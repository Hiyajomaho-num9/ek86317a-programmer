/**
 * ConfigTab — Configuration registers panel.
 * 11 register panels arranged in a CSS Grid layout (2~3 columns).
 * Each panel uses RegisterPanel + BitField for bit-level configuration.
 */

import { useAppContext } from '../../App';
import RegisterPanel from './RegisterPanel';
import BitField from './BitField';

// ============================================================================
// Register Address Constants for Config registers
// ============================================================================

const REG_CH_ON_OFF    = 0x28;
const REG_CH_SET       = 0x29;
const REG_DELAY1       = 0x2A;
const REG_DELAY2       = 0x2B;
const REG_DIS_RES      = 0x2C;
const REG_CONFIG1      = 0x2D;
const REG_VGH_VGL_VTC  = 0x2E;
const REG_VCM_VTC      = 0x2F;
const REG_CONFIG2      = 0x30;
const REG_CONFIG3      = 0x31;
const REG_DISCHARGE2   = 0x32;
const REG_CONFIG4      = 0x46;

// ============================================================================
// Helper: bit manipulation on register value
// ============================================================================

/** Get single bit value */
function getBit(regVal: number, bit: number): boolean {
  return ((regVal >> bit) & 1) === 1;
}

/** Set single bit, return new register value */
function setBit(regVal: number, bit: number, on: boolean): number {
  if (on) return regVal | (1 << bit);
  return regVal & ~(1 << bit);
}

/** Get multi-bit field value: extract bits [high:low] */
function getField(regVal: number, shift: number, width: number): number {
  const mask = (1 << width) - 1;
  return (regVal >> shift) & mask;
}

/** Set multi-bit field value: replace bits [shift+width-1:shift] with fieldVal */
function setField(regVal: number, shift: number, width: number, fieldVal: number): number {
  const mask = (1 << width) - 1;
  return (regVal & ~(mask << shift)) | ((fieldVal & mask) << shift);
}

// ============================================================================
// VTC options generator (shared by VGH_VGL_VTC and VCM_VTC panels)
// ============================================================================

function generateVtcOptions(): { value: number; label: string }[] {
  const opts: { value: number; label: string }[] = [];
  for (let i = 0; i <= 0x0F; i++) {
    const voltage = 0.4 + i * 0.2;
    opts.push({ value: i, label: `${voltage.toFixed(1)}V` });
  }
  return opts;
}

const VTC_OPTIONS = generateVtcOptions();

// ============================================================================
// XON_DIS_THR options generator
// ============================================================================

function generateXonDisThrOptions(): { value: number; label: string }[] {
  const opts: { value: number; label: string }[] = [];
  for (let i = 0; i <= 0x0F; i++) {
    const voltage = 6.50 + i * 0.25;
    opts.push({ value: i, label: `${voltage.toFixed(2)}V` });
  }
  return opts;
}

const XON_DIS_THR_OPTIONS = generateXonDisThrOptions();

// ============================================================================
// ConfigTab Component
// ============================================================================

function ConfigTab() {
  const ctx = useAppContext();
  const isLp = ctx.chipModel === 'lp6281';
  const supportsMntMode = ctx.chipCapabilities.supportsMntMode;

  // Helper to get register value with fallback
  const regVal = (addr: number) => ctx.getDacValue(addr) ?? 0;

  // Helper to update register value
  const setReg = (addr: number, val: number) => ctx.setDacValue(addr, val);

  // Shorthand for single-bit checkbox change
  const onBitChange = (addr: number, bit: number) => (checked: boolean) => {
    setReg(addr, setBit(regVal(addr), bit, checked));
  };

  // Shorthand for multi-bit dropdown change
  const onFieldChange = (addr: number, shift: number, width: number) => (value: number) => {
    setReg(addr, setField(regVal(addr), shift, width, value));
  };

  // Enable All Channels handler
  const handleEnableAll = async () => {
    try {
      setReg(REG_CH_ON_OFF, 0xFF);
      await ctx.writeDacRegister(REG_CH_ON_OFF, 0xFF);
      ctx.addLog('success', 'All channels enabled (0x28 = 0xFF)');
    } catch (e: unknown) {
      ctx.addLog('error', `Enable all channels failed: ${e}`);
    }
  };

  const btnSmall = 'px-2 py-1 text-xs rounded text-white font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed';

  return (
    <div className="space-y-3">
      <h2 className="text-lg font-semibold text-gray-200">Configuration Registers</h2>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">

        {/* ============================================================
            Panel 1: Channel On/Off Set (0x28)
            ============================================================ */}
        <RegisterPanel
          title="Channel On/Off Set (28h)"
          address={REG_CH_ON_OFF}
          extraButtons={
            <button
              onClick={handleEnableAll}
              className={`${btnSmall} bg-green-600 hover:bg-green-700 flex-1`}
              disabled={!ctx.connected}
            >
              Enable All Channels
            </button>
          }
        >
          <BitField label="b7: Group_B_EN" type="checkbox" checked={getBit(regVal(REG_CH_ON_OFF), 7)} onCheckChange={onBitChange(REG_CH_ON_OFF, 7)} />
          <BitField label="b6: VGL_EN" type="checkbox" checked={getBit(regVal(REG_CH_ON_OFF), 6)} onCheckChange={onBitChange(REG_CH_ON_OFF, 6)} />
          <BitField label="b5: VSS_EN" type="checkbox" checked={getBit(regVal(REG_CH_ON_OFF), 5)} onCheckChange={onBitChange(REG_CH_ON_OFF, 5)} />
          <BitField label="b4: AVDD_EN" type="checkbox" checked={getBit(regVal(REG_CH_ON_OFF), 4)} onCheckChange={onBitChange(REG_CH_ON_OFF, 4)} />
          <BitField label="b3: HAVDD_EN" type="checkbox" checked={getBit(regVal(REG_CH_ON_OFF), 3)} onCheckChange={onBitChange(REG_CH_ON_OFF, 3)} />
          <BitField label="b2: GAM_EN" type="checkbox" checked={getBit(regVal(REG_CH_ON_OFF), 2)} onCheckChange={onBitChange(REG_CH_ON_OFF, 2)} />
          <BitField label="b1: VCOM1_EN" type="checkbox" checked={getBit(regVal(REG_CH_ON_OFF), 1)} onCheckChange={onBitChange(REG_CH_ON_OFF, 1)} />
          <BitField label="b0: VGH_EN" type="checkbox" checked={getBit(regVal(REG_CH_ON_OFF), 0)} onCheckChange={onBitChange(REG_CH_ON_OFF, 0)} />
        </RegisterPanel>

        {/* ============================================================
            Panel 2: Channel Set (0x29)
            ============================================================ */}
        <RegisterPanel title="Channel Set (29h)" address={REG_CH_SET}>
          <BitField label="b7: FREQ_SEL (0=750kHz, 1=500kHz)" type="checkbox" checked={getBit(regVal(REG_CH_SET), 7)} onCheckChange={onBitChange(REG_CH_SET, 7)} />
          <BitField label="b4: AVDD_NMOS (0=Int, 1=Ext)" type="checkbox" checked={getBit(regVal(REG_CH_SET), 4)} onCheckChange={onBitChange(REG_CH_SET, 4)} />
          <BitField label="b3: VGH_TYPE (0=Boost, 1=CP)" type="checkbox" checked={getBit(regVal(REG_CH_SET), 3)} onCheckChange={onBitChange(REG_CH_SET, 3)} />
          <BitField label="b2: VGL_TYPE (0=Inv, 1=CP)" type="checkbox" checked={getBit(regVal(REG_CH_SET), 2)} onCheckChange={onBitChange(REG_CH_SET, 2)} />
          <BitField label="b1: VGL_PUMP_SRC (0=LXBK1, 1=LX/CS)" type="checkbox" checked={getBit(regVal(REG_CH_SET), 1)} onCheckChange={onBitChange(REG_CH_SET, 1)} />
          <BitField label="b0: GAM_CH_TYPE (0=14CH, 1=4CH)" type="checkbox" checked={getBit(regVal(REG_CH_SET), 0)} onCheckChange={onBitChange(REG_CH_SET, 0)} />
        </RegisterPanel>

        {/* ============================================================
            Panel 3: Delay Time Set 1 (0x2A)
            ============================================================ */}
        <RegisterPanel title="Delay Time Set 1 (2Ah)" address={REG_DELAY1}>
          <BitField
            label="[b5:b4] VBK1_DLY"
            type="dropdown"
            options={[
              { value: 0, label: '0ms' },
              { value: 1, label: '2ms' },
              { value: 2, label: '4ms' },
              { value: 3, label: '6ms' },
            ]}
            selectedValue={getField(regVal(REG_DELAY1), 4, 2)}
            onSelectChange={onFieldChange(REG_DELAY1, 4, 2)}
          />
          <BitField
            label="[b3:b2] VGL_DLY"
            type="dropdown"
            options={[
              { value: 0, label: '0ms' },
              { value: 1, label: '5ms' },
              { value: 2, label: '10ms' },
              { value: 3, label: '15ms' },
            ]}
            selectedValue={getField(regVal(REG_DELAY1), 2, 2)}
            onSelectChange={onFieldChange(REG_DELAY1, 2, 2)}
          />
          <BitField
            label="[b1:b0] VSS1_DLY"
            type="dropdown"
            options={[
              { value: 0, label: '0ms' },
              { value: 1, label: '2ms' },
              { value: 2, label: '4ms' },
              { value: 3, label: '6ms' },
            ]}
            selectedValue={getField(regVal(REG_DELAY1), 0, 2)}
            onSelectChange={onFieldChange(REG_DELAY1, 0, 2)}
          />
        </RegisterPanel>

        {/* ============================================================
            Panel 4: Delay Time Set 2 (0x2B)
            ============================================================ */}
        <RegisterPanel title="Delay Time Set 2 (2Bh)" address={REG_DELAY2}>
          <BitField
            label="[b6:b5] AVDD_DLY"
            type="dropdown"
            options={[
              { value: 0, label: '0ms' },
              { value: 1, label: '5ms' },
              { value: 2, label: '10ms' },
              { value: 3, label: '15ms' },
            ]}
            selectedValue={getField(regVal(REG_DELAY2), 5, 2)}
            onSelectChange={onFieldChange(REG_DELAY2, 5, 2)}
          />
          <BitField
            label="[b4:b3] VGH_DLY"
            type="dropdown"
            options={[
              { value: 0, label: '0ms' },
              { value: 1, label: '2ms' },
              { value: 2, label: '4ms' },
              { value: 3, label: '6ms' },
            ]}
            selectedValue={getField(regVal(REG_DELAY2), 3, 2)}
            onSelectChange={onFieldChange(REG_DELAY2, 3, 2)}
          />
          <BitField
            label="[b2:b0] VCOM_DLY"
            type="dropdown"
            options={[
              { value: 0, label: '0ms' },
              { value: 1, label: '30ms' },
              { value: 2, label: '60ms' },
              { value: 3, label: '90ms' },
              { value: 4, label: '120ms' },
              { value: 5, label: '150ms' },
              { value: 6, label: '180ms' },
              { value: 7, label: '210ms' },
            ]}
            selectedValue={getField(regVal(REG_DELAY2), 0, 3)}
            onSelectChange={onFieldChange(REG_DELAY2, 0, 3)}
          />
        </RegisterPanel>

        {/* ============================================================
            Panel 5: Discharge Resistor Set (0x2C)
            ============================================================ */}
        <RegisterPanel title="Discharge Resistor Set (2Ch)" address={REG_DIS_RES}>
          <BitField label="b7: ALL_DIS_EN (0=Dis, 1=En)" type="checkbox" checked={getBit(regVal(REG_DIS_RES), 7)} onCheckChange={onBitChange(REG_DIS_RES, 7)} />
          <BitField label="b6: BK1_DIS (0=0.5kΩ, 1=1kΩ)" type="checkbox" checked={getBit(regVal(REG_DIS_RES), 6)} onCheckChange={onBitChange(REG_DIS_RES, 6)} />
          <BitField label="b5: AVDD_DIS (0=1.4kΩ, 1=4.7kΩ)" type="checkbox" checked={getBit(regVal(REG_DIS_RES), 5)} onCheckChange={onBitChange(REG_DIS_RES, 5)} />
          <BitField label="b4: VGH_DIS (0=1.5kΩ, 1=30kΩ)" type="checkbox" checked={getBit(regVal(REG_DIS_RES), 4)} onCheckChange={onBitChange(REG_DIS_RES, 4)} />
          <BitField label="b3: VGL_DIS (0=Dis, 1=10kΩ)" type="checkbox" checked={getBit(regVal(REG_DIS_RES), 3)} onCheckChange={onBitChange(REG_DIS_RES, 3)} />
          <BitField label="b2: HAVDD_DIS (0=Dis, 1=10kΩ)" type="checkbox" checked={getBit(regVal(REG_DIS_RES), 2)} onCheckChange={onBitChange(REG_DIS_RES, 2)} />
          <BitField
            label="[b1:b0] VCOM1_DIS"
            type="dropdown"
            options={[
              { value: 0, label: 'Disable' },
              { value: 1, label: '20Ω' },
              { value: 2, label: '1kΩ' },
              { value: 3, label: '8kΩ' },
            ]}
            selectedValue={getField(regVal(REG_DIS_RES), 0, 2)}
            onSelectChange={onFieldChange(REG_DIS_RES, 0, 2)}
          />
        </RegisterPanel>

        {/* ============================================================
            Panel 6: Configuration 1 (0x2D)
            ============================================================ */}
        <RegisterPanel title="Configuration 1 (2Dh)" address={REG_CONFIG1}>
          <BitField label="b7: VSS_DIS (0=Dis, 1=1.2kΩ)" type="checkbox" checked={getBit(regVal(REG_CONFIG1), 7)} onCheckChange={onBitChange(REG_CONFIG1, 7)} />
          <BitField label="b4: VCOM1_DIS_TYPE (0=UVLO, 1=XON)" type="checkbox" checked={getBit(regVal(REG_CONFIG1), 4)} onCheckChange={onBitChange(REG_CONFIG1, 4)} />
          <BitField label="b3: HAVDD_FB (0=GAM avg, 1=DAC)" type="checkbox" checked={getBit(regVal(REG_CONFIG1), 3)} onCheckChange={onBitChange(REG_CONFIG1, 3)} />
          <BitField label="b2: AVDD_GD_SC (0=3 fail, 1=3+V)" type="checkbox" checked={getBit(regVal(REG_CONFIG1), 2)} onCheckChange={onBitChange(REG_CONFIG1, 2)} />
          <BitField label="b1: AVDD_SS (0=10ms, 1=20ms)" type="checkbox" checked={getBit(regVal(REG_CONFIG1), 1)} onCheckChange={onBitChange(REG_CONFIG1, 1)} />
          <BitField label="b0: VGH_SS (0=3ms, 1=6ms)" type="checkbox" checked={getBit(regVal(REG_CONFIG1), 0)} onCheckChange={onBitChange(REG_CONFIG1, 0)} />
        </RegisterPanel>

        {/* ============================================================
            Panel 7: VGH_VGL_VTC1/2 (0x2E)
            ============================================================ */}
        <RegisterPanel title="VGH_VGL_VTC1/2 (2Eh)" address={REG_VGH_VGL_VTC}>
          <BitField
            label="[b7:b4] VTC2"
            type="dropdown"
            options={VTC_OPTIONS}
            selectedValue={getField(regVal(REG_VGH_VGL_VTC), 4, 4)}
            onSelectChange={onFieldChange(REG_VGH_VGL_VTC, 4, 4)}
          />
          <BitField
            label="[b3:b0] VTC1"
            type="dropdown"
            options={VTC_OPTIONS}
            selectedValue={getField(regVal(REG_VGH_VGL_VTC), 0, 4)}
            onSelectChange={onFieldChange(REG_VGH_VGL_VTC, 0, 4)}
          />
        </RegisterPanel>

        {/* ============================================================
            Panel 8: VCM_VTC1/2 (0x2F)
            ============================================================ */}
        <RegisterPanel title="VCM_VTC1/2 (2Fh)" address={REG_VCM_VTC}>
          <BitField
            label="[b7:b4] VCOM1_VTC2"
            type="dropdown"
            options={VTC_OPTIONS}
            selectedValue={getField(regVal(REG_VCM_VTC), 4, 4)}
            onSelectChange={onFieldChange(REG_VCM_VTC, 4, 4)}
          />
          <BitField
            label="[b3:b0] VCOM1_VTC1"
            type="dropdown"
            options={VTC_OPTIONS}
            selectedValue={getField(regVal(REG_VCM_VTC), 0, 4)}
            onSelectChange={onFieldChange(REG_VCM_VTC, 0, 4)}
          />
        </RegisterPanel>

        {/* ============================================================
            Panel 9: Configuration 2 (0x30)
            ============================================================ */}
        <RegisterPanel title="Configuration 2 (30h)" address={REG_CONFIG2}>
          <BitField
            label="[b5:b4] AVDD_EXT_DRV"
            type="dropdown"
            options={[
              { value: 0, label: 'EXT_DRV_1' },
              { value: 1, label: 'EXT_DRV_2' },
              { value: 2, label: 'EXT_DRV_3' },
              { value: 3, label: 'EXT_DRV_4' },
            ]}
            selectedValue={getField(regVal(REG_CONFIG2), 4, 2)}
            onSelectChange={onFieldChange(REG_CONFIG2, 4, 2)}
          />
          <BitField
            label="[b3:b2] VGH_BOOST_COMP"
            type="dropdown"
            options={[
              { value: 0, label: 'COMP_1' },
              { value: 1, label: 'COMP_2' },
              { value: 2, label: 'COMP_3' },
              { value: 3, label: 'COMP_4' },
            ]}
            selectedValue={getField(regVal(REG_CONFIG2), 2, 2)}
            onSelectChange={onFieldChange(REG_CONFIG2, 2, 2)}
          />
          <BitField
            label="[b1:b0] VGL_INV_COMP"
            type="dropdown"
            options={[
              { value: 0, label: 'COMP_1' },
              { value: 1, label: 'COMP_2' },
              { value: 2, label: 'COMP_3' },
              { value: 3, label: 'COMP_4' },
            ]}
            selectedValue={getField(regVal(REG_CONFIG2), 0, 2)}
            onSelectChange={onFieldChange(REG_CONFIG2, 0, 2)}
          />
        </RegisterPanel>

        {!isLp && (
          <>
            {/* ============================================================
                Panel 10: Configuration 3 (0x31)
                ============================================================ */}
            <RegisterPanel title="Configuration 3 (31h)" address={REG_CONFIG3}>
              <BitField label="b7: EN_PIN_SEL (0=EN in, 1=XON out)" type="checkbox" checked={getBit(regVal(REG_CONFIG3), 7)} onCheckChange={onBitChange(REG_CONFIG3, 7)} />
              <BitField label="b6: VCOM1_DLY_OFF" type="checkbox" checked={getBit(regVal(REG_CONFIG3), 6)} onCheckChange={onBitChange(REG_CONFIG3, 6)} />
              <BitField label="b5: HAVDD_DLY_OFF" type="checkbox" checked={getBit(regVal(REG_CONFIG3), 5)} onCheckChange={onBitChange(REG_CONFIG3, 5)} />
              <BitField label="b4: GAM_DLY_OFF" type="checkbox" checked={getBit(regVal(REG_CONFIG3), 4)} onCheckChange={onBitChange(REG_CONFIG3, 4)} />
              <BitField
                label="[b3:b0] XON_DIS_THR"
                type="dropdown"
                options={XON_DIS_THR_OPTIONS}
                selectedValue={getField(regVal(REG_CONFIG3), 0, 4)}
                onSelectChange={onFieldChange(REG_CONFIG3, 0, 4)}
              />
            </RegisterPanel>

            {/* ============================================================
                Panel 11: Discharge Disable Set (0x32)
                ============================================================ */}
            <RegisterPanel title="Discharge Disable Set (32h)" address={REG_DISCHARGE2}>
              <BitField label="b2: VBK1_DISA (0=Enable, 1=Disable)" type="checkbox" checked={getBit(regVal(REG_DISCHARGE2), 2)} onCheckChange={onBitChange(REG_DISCHARGE2, 2)} />
              <BitField label="b1: AVDD_DIS_DISA (0=Enable, 1=Disable)" type="checkbox" checked={getBit(regVal(REG_DISCHARGE2), 1)} onCheckChange={onBitChange(REG_DISCHARGE2, 1)} />
              <BitField label="b0: VGH_DIS_DISA (0=Enable, 1=Disable)" type="checkbox" checked={getBit(regVal(REG_DISCHARGE2), 0)} onCheckChange={onBitChange(REG_DISCHARGE2, 0)} />
            </RegisterPanel>

            {/* ============================================================
                Panel 12: Configuration 4 / VCOM2DAC (0x46)
                ============================================================ */}
            <RegisterPanel title={supportsMntMode ? "Configuration 4 / MNT Mode (46h)" : "VCOM2DAC Control (46h)"} address={REG_CONFIG4}>
              <BitField label="b0: VCOM2DAC_EN" type="checkbox" checked={getBit(regVal(REG_CONFIG4), 0)} onCheckChange={onBitChange(REG_CONFIG4, 0)} />
              {supportsMntMode && (
                <BitField label="b7: MNT_MODE_EN (0=TV, 1=MNT)" type="checkbox" checked={getBit(regVal(REG_CONFIG4), 7)} onCheckChange={onBitChange(REG_CONFIG4, 7)} />
              )}
            </RegisterPanel>
          </>
        )}

      </div>
    </div>
  );
}

export default ConfigTab;
