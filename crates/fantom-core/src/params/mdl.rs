//! MODEL (`MDLa`) engine parameter tables: the analog `[MDLSYN0]` block, JD-800 and WT.
//!
//! Offsets, sizes and ranges from the firmware's parameter descriptors; offsets are
//! record-relative.

use super::{MdlCurve, MdlMirror, MdlParam, MdlTarget};

/// On-disk length of one `MDLa` record.
pub const RECORD_LEN: usize = 2048;
/// Where the engine block starts in a record.
pub const ENGINE_BASE: usize = 0x670;

#[rustfmt::skip]
pub static ANALOG: &[MdlParam] = &[
    MdlParam { id: "Model", offset: 1648, size: 1, min: 0, max: 4, labels: &["---", "JP8", "JX8P", "JUNO106", "SH101"], mirror: MdlMirror::Locked },
    MdlParam { id: "LFO WAVE FORM", offset: 1649, size: 1, min: 0, max: 10, labels: &["SIN", "TRI", "SAW-UP", "SAW-DW", "SQR", "RND", "TRP", "S&H", "CHS", "VSIN", "STEP"], mirror: MdlMirror::Mirrored },
    MdlParam { id: "LFO RATE", offset: 1650, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "LFO DELAY TIME", offset: 1652, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "OSC1 MOD", offset: 1654, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC1 PITCH ENV DEPTH", offset: 1655, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC1 DEST SELECT", offset: 1656, size: 1, min: 0, max: 2, labels: &["OSC1", "BOTH", "OSC2"], mirror: MdlMirror::Mirrored },
    MdlParam { id: "OSC1 ENVT SELECT", offset: 1657, size: 1, min: 0, max: 1, labels: &["1", "2"], mirror: MdlMirror::Free },
    MdlParam { id: "OSC1 WAVE FORM", offset: 1658, size: 2, min: 0, max: 27, labels: &["PW", "SAW", "SUB1OCT1", "SUB2OCT1", "SUB2OCT2", "NOISE", "PW", "SAW", "SUB1OCT1", "NOISE", "SIN", "TRI", "SAW", "PW", "SQR", "NOISE", "SAW", "PULSE", "SQR", "NOISE", "TRI", "SAW+TRI", "SAW", "SQR", "SQR2", "SQR3", "WHITE NOISE", "PINK NOISE"], mirror: MdlMirror::Mirrored },
    MdlParam { id: "OSC1 FEET", offset: 1660, size: 1, min: 0, max: 4, labels: &["32'", "16'", "8'", "4'", "2'"], mirror: MdlMirror::Mirrored },
    MdlParam { id: "OSC1 COARSE TUNE", offset: 1661, size: 1, min: -48, max: 48, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "OSC1 FINE TUNE", offset: 1662, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "OSC1 LOW FREQ PITCH", offset: 1663, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "OSC1 MODE", offset: 1664, size: 1, min: 0, max: 1, labels: &["NORMAL", "LOW FREQ"], mirror: MdlMirror::Free },
    MdlParam { id: "OSC1 PWM DEPTH", offset: 1665, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "OSC1 PULSE WIDTH", offset: 1666, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC1 PULSE WIDTH MODE", offset: 1667, size: 1, min: 0, max: 2, labels: &["LFO", "MANUAL", "ENV"], mirror: MdlMirror::Mirrored },
    MdlParam { id: "OSC2 MOD", offset: 1668, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "OSC2 PITCH ENV DEPTH", offset: 1669, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "OSC2 WAVE FORM", offset: 1670, size: 2, min: 0, max: 27, labels: &["PW", "SAW", "SUB1OCT1", "SUB2OCT1", "SUB2OCT2", "NOISE", "PW", "SAW", "SUB1OCT1", "NOISE", "SIN", "TRI", "SAW", "PW", "SQR", "NOISE", "SAW", "PULSE", "SQR", "NOISE", "TRI", "SAW+TRI", "SAW", "SQR", "SQR2", "SQR3", "WHITE NOISE", "PINK NOISE"], mirror: MdlMirror::Mirrored },
    MdlParam { id: "OSC2 FEET", offset: 1672, size: 1, min: 0, max: 4, labels: &["32'", "16'", "8'", "4'", "2'"], mirror: MdlMirror::Free },
    MdlParam { id: "OSC2 COARSE TUNE", offset: 1673, size: 1, min: -48, max: 48, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "OSC2 FINE TUNE", offset: 1674, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "OSC2 LOW FREQ PITCH", offset: 1675, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC2 MODE", offset: 1676, size: 1, min: 0, max: 1, labels: &["NORMAL", "LO FREQ"], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC2 PULSE WIDTH", offset: 1677, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "OSC3 MOD", offset: 1678, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "OSC3 PITCH ENV DEPTH", offset: 1679, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "OSC3 WAVE FORM", offset: 1680, size: 2, min: 0, max: 27, labels: &["PW", "SAW", "SQR 1OCT", "SQR 2OCT", "SH SubOSC", "NOISE", "PW", "SAW", "SQR 1OCT", "NOISE", "SIN", "TRI", "SAW", "PW", "SQR", "NOISE", "SAW", "PULSE", "SQR", "NOISE", "TRI", "SAW+TRI", "SAW", "SQR", "SQR2", "SQR3", "WHITE NOISE", "PINK NOISE"], mirror: MdlMirror::Free },
    MdlParam { id: "OSC3 FEET", offset: 1682, size: 1, min: 0, max: 4, labels: &["32'", "16'", "8'", "4'", "2'"], mirror: MdlMirror::Free },
    MdlParam { id: "OSC3 COARSE TUNE", offset: 1683, size: 1, min: -48, max: 48, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "OSC3 FINE TUNE", offset: 1684, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "OSC3 LOW FREQ PITCH", offset: 1685, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "OSC3 MODE", offset: 1686, size: 1, min: 0, max: 1, labels: &["NORMAL", "LO FREQ"], mirror: MdlMirror::Free },
    MdlParam { id: "NOISE", offset: 1687, size: 1, min: 0, max: 1, labels: &["WHITE", "PINK"], mirror: MdlMirror::Free },
    MdlParam { id: "MOD DEPTH", offset: 1688, size: 2, min: 0, max: 10800, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "MOD MODE", offset: 1690, size: 1, min: 0, max: 3, labels: &["OFF", "SYNC", "RING", "X-MOD"], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC1 LEVEL", offset: 1691, size: 1, min: 0, max: 255, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "OSC2 LEVEL", offset: 1692, size: 1, min: 0, max: 255, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "OSC3 LEVEL", offset: 1693, size: 1, min: 0, max: 255, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "NOISE LEVEL", offset: 1694, size: 1, min: 0, max: 255, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "MIXER ENV DEPTH", offset: 1695, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "MIXER ENV SELECT", offset: 1696, size: 1, min: 0, max: 1, labels: &["1", "2"], mirror: MdlMirror::Free },
    MdlParam { id: "MIXER DYNAMICS", offset: 1697, size: 1, min: 0, max: 3, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "FILTER TYPE", offset: 1698, size: 1, min: 0, max: 3, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "FILTER HPF STEP", offset: 1699, size: 1, min: 0, max: 3, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "FILTER HPF", offset: 1700, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "FILTER FREQ", offset: 1702, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "FILTER RESO", offset: 1704, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "FILTER ENV DEPTH", offset: 1706, size: 2, min: -1023, max: 1023, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "FILTER ENV SELECT", offset: 1708, size: 1, min: 0, max: 1, labels: &["1", "2"], mirror: MdlMirror::Unproven },
    MdlParam { id: "FILTER SLOPE", offset: 1709, size: 1, min: 0, max: 2, labels: &["-12dB", "-18db", "-24dB"], mirror: MdlMirror::Mirrored },
    MdlParam { id: "FILTER KEY FOLLOW", offset: 1710, size: 2, min: -200, max: 200, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "FILTER MOD", offset: 1712, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "AMP LEVEL", offset: 1714, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "AMP MOD", offset: 1715, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "AMP MOD STEP", offset: 1716, size: 1, min: 0, max: 3, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "AMP ENV SELECT", offset: 1717, size: 1, min: 0, max: 3, labels: &["ENV F&A", "ENV2", "G-AMP", "G-ENV2"], mirror: MdlMirror::Unproven },
    MdlParam { id: "ENV1 ATTACK", offset: 1718, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "ENV1 DECAY", offset: 1720, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "ENV1 SUSTAIN", offset: 1722, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "ENV1 RELEASE", offset: 1724, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "ENV1 KEY FOLLOW", offset: 1726, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "ENV1 KEY FOLLOW STEP", offset: 1727, size: 1, min: 0, max: 3, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "ENV2 ATTACK", offset: 1728, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "ENV2 DECAY", offset: 1730, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "ENV2 SUSTAIN", offset: 1732, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "ENV2 RELEASE", offset: 1734, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "ENV2 MODE", offset: 1736, size: 1, min: 0, max: 2, labels: &["GATE+TRIG", "GATE", "LFO"], mirror: MdlMirror::Free },
    MdlParam { id: "ENV2 KEY FOLLOW", offset: 1737, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "ENV2 KEY FOLLOW STEP", offset: 1738, size: 1, min: 0, max: 3, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "DYNAMICS PITCH", offset: 1739, size: 1, min: 0, max: 3, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "DYNAMICS FILTER", offset: 1740, size: 1, min: 0, max: 3, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "DYNAMICS AMP", offset: 1741, size: 1, min: 0, max: 3, labels: &[], mirror: MdlMirror::Free },
    MdlParam { id: "PITCH DRIFT", offset: 1742, size: 1, min: 0, max: 255, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "BENDER PITCH DEPTH STEP", offset: 1743, size: 1, min: 0, max: 3, labels: &["2", "3", "4", "7"], mirror: MdlMirror::Free },
    MdlParam { id: "BENDER FILTER DEPTH", offset: 1744, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "MODULATION LFO DEPTH", offset: 1745, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "BENDER PITCH DEPTH", offset: 1746, size: 2, min: 0, max: 4800, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PORTAMENT TIME", offset: 1748, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "PORTAMENT MODE", offset: 1750, size: 1, min: 0, max: 2, labels: &["OFF", "ON", "AUTO"], mirror: MdlMirror::Unproven },
    MdlParam { id: "KEY MODE", offset: 1751, size: 1, min: 0, max: 3, labels: &["POLY", "SOLO", "UNISON", "SL-UNISON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PARAMETER EXPANSION SWITCH", offset: 1752, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "CONDITION", offset: 1753, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Mirrored },
    MdlParam { id: "PORTAMENT CURVE TYPE", offset: 1754, size: 1, min: 0, max: 3, labels: &["ORIGINAL", "LINER", "EXP1", "EXP2"], mirror: MdlMirror::Unproven },
    MdlParam { id: "AFTER TOUCH SENS LFO", offset: 1755, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "AFTER TOUCH SENS FREQ", offset: 1756, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "AFTER TOUCH SENS LEVEL", offset: 1757, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
];

#[rustfmt::skip]
pub static JD800: &[MdlParam] = &[
    MdlParam { id: "PTL1 Layer Partial Switch", offset: 1648, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 Active Partial Switch", offset: 1649, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 Velocity Curve", offset: 1650, size: 1, min: 0, max: 3, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 Hold Control Switch", offset: 1651, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 LFO1:Waveform", offset: 1652, size: 1, min: 0, max: 4, labels: &["TRI", "SAW", "SQU", "S&H", "RND"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 LFO1:Rate sync", offset: 1653, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 LFO1:Rate", offset: 1654, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 LFO1:Rate nt", offset: 1655, size: 1, min: 0, max: 22, labels: &["1/64T", "1/64", "1/32T", "1/32", "1/16T", "1/32.", "1/16", "1/8T", "1/16.", "1/8", "1/4T", "1/8.", "1/4", "1/2T", "1/4.", "1/2", "1T", "1/2.", "1", "2T", "1.", "2", "4"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 LFO1:Delay time", offset: 1656, size: 1, min: 0, max: 101, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 LFO1:Fade time", offset: 1657, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 LFO1:Offset", offset: 1658, size: 1, min: 0, max: 2, labels: &["-", "0", "+"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 LFO1:Key trig", offset: 1659, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 LFO2:Waveform", offset: 1660, size: 1, min: 0, max: 4, labels: &["TRI", "SAW", "SQU", "S&H", "RND"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 LFO2:Rate Sync", offset: 1661, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 LFO2:Rate", offset: 1662, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 LFO2:Rate nt", offset: 1663, size: 1, min: 0, max: 22, labels: &["1/64T", "1/64", "1/32T", "1/32", "1/16T", "1/32.", "1/16", "1/8T", "1/16.", "1/8", "1/4T", "1/8.", "1/4", "1/2T", "1/4.", "1/2", "1T", "1/2.", "1", "2T", "1.", "2", "4"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 LFO2:Delay Time", offset: 1664, size: 1, min: 0, max: 101, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 LFO2:Fade time", offset: 1665, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 LFO2:Offset", offset: 1666, size: 1, min: 0, max: 2, labels: &["-", "0", "+"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 LFO2:Key trig", offset: 1667, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 WG:Waveform", offset: 1668, size: 2, min: 0, max: 108, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 Reserved", offset: 1670, size: 1, min: 0, max: 0, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 WG:Wave gain", offset: 1671, size: 1, min: 0, max: 5, labels: &["-18", "-12", "-6", "0", "+6", "+12[dB]"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 WG:Pitch coarse", offset: 1672, size: 1, min: -48, max: 48, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 WG:Pitch fine", offset: 1673, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 WG:Pitch random", offset: 1674, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 WG:Pitch KF", offset: 1675, size: 1, min: 0, max: 16, labels: &["-100", "-50", "-20", "-10", "-5", "0", "+5", "+10", "+20", "+50", "+98", "+99", "+100", "+101", "+102", "+150", "+200[%]"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 WG:Bender Sw", offset: 1676, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 WG:Atch bend sw", offset: 1677, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 WG:LFO1 depth", offset: 1678, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 WG:LFO2 depth", offset: 1679, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 WG:Lever sens", offset: 1680, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 WG:Atch msens", offset: 1681, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 Pitch Env Velocity Sens", offset: 1682, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 Pitch Env Time Velocity Sens", offset: 1683, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 Pitch Env Time Keyfollow", offset: 1684, size: 1, min: -10, max: 10, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 Pitch Env L0", offset: 1685, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 Pitch Env L1", offset: 1686, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 Pitch Env L2", offset: 1687, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 Pitch Env T1", offset: 1688, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 Pitch Env T2", offset: 1689, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 Pitch Env T3", offset: 1690, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 TVF:Filter mode", offset: 1691, size: 1, min: 0, max: 2, labels: &["LPF", "BPF", "HPF"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 TVF:Cutoff freq", offset: 1692, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 TVF:Resonance", offset: 1693, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 TVF:Cutoff KF", offset: 1694, size: 1, min: 0, max: 40, labels: &["-100", "-90", "-80", "-70", "-60", "-50", "-40", "-30", "-20", "-10", "0", "+5", "+10", "+15", "+20", "+25", "+30", "+35", "+40", "+45", "+50", "+55", "+60", "+65", "+70", "+75", "+80", "+85", "+90", "+95", "+100", "+105", "+110", "+115", "+120", "+125", "+130", "+135", "+140", "+145", "+150[%]"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 TVF:Atch sens", offset: 1695, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 TVF:LFO select", offset: 1696, size: 1, min: 0, max: 1, labels: &["LFO1", "LFO2"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 TVF:LFO depth", offset: 1697, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 TVF:ENV depth", offset: 1698, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 F-ENV:Velo sens", offset: 1699, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 F-ENV:T vsens", offset: 1700, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 F-ENV:Time KF", offset: 1701, size: 1, min: -10, max: 10, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 F-ENV:Level 1", offset: 1702, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 F-ENV:Level 2", offset: 1703, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 F-ENV:Sus Level", offset: 1704, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 F-ENV:Level 4", offset: 1705, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 F-ENV:Time 1", offset: 1706, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 F-ENV:Time 2", offset: 1707, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 F-ENV:Time 3", offset: 1708, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 F-ENV:Time 4", offset: 1709, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 TVA Bias Direction", offset: 1710, size: 1, min: 0, max: 2, labels: &["UP", "LOW", "U&L"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 TVA Bias Point", offset: 1711, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 TVA:Bias level", offset: 1712, size: 1, min: -10, max: 10, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 TVA:Level", offset: 1713, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 TVA After Touch Sens", offset: 1714, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 TVA:LFO select", offset: 1715, size: 1, min: 0, max: 1, labels: &["LFO1", "LFO2"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 TVA:LFO depth", offset: 1716, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 A-ENV:Velo sens", offset: 1717, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 A-ENV:T vsens", offset: 1718, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 A-ENV:Time KF", offset: 1719, size: 1, min: -10, max: 10, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 A-ENV:Level 1", offset: 1720, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 A-ENV:Level 2", offset: 1721, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 A-ENV:Sus level", offset: 1722, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 A-ENV:Time 1", offset: 1723, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 A-ENV:Time 2", offset: 1724, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 A-ENV:Time 3", offset: 1725, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL1 A-ENV:Time 4", offset: 1726, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 Layer Partial Switch", offset: 1728, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 Active Partial Switch", offset: 1729, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 Velocity Curve", offset: 1730, size: 1, min: 0, max: 3, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 Hold Control Switch", offset: 1731, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 LFO1:Waveform", offset: 1732, size: 1, min: 0, max: 4, labels: &["TRI", "SAW", "SQU", "S&H", "RND"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 LFO1:Rate sync", offset: 1733, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 LFO1:Rate", offset: 1734, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 LFO1:Rate nt", offset: 1735, size: 1, min: 0, max: 22, labels: &["1/64T", "1/64", "1/32T", "1/32", "1/16T", "1/32.", "1/16", "1/8T", "1/16.", "1/8", "1/4T", "1/8.", "1/4", "1/2T", "1/4.", "1/2", "1T", "1/2.", "1", "2T", "1.", "2", "4"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 LFO1:Delay time", offset: 1736, size: 1, min: 0, max: 101, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 LFO1:Fade time", offset: 1737, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 LFO1:Offset", offset: 1738, size: 1, min: 0, max: 2, labels: &["-", "0", "+"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 LFO1:Key trig", offset: 1739, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 LFO2:Waveform", offset: 1740, size: 1, min: 0, max: 4, labels: &["TRI", "SAW", "SQU", "S&H", "RND"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 LFO2:Rate Sync", offset: 1741, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 LFO2:Rate", offset: 1742, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 LFO2:Rate nt", offset: 1743, size: 1, min: 0, max: 22, labels: &["1/64T", "1/64", "1/32T", "1/32", "1/16T", "1/32.", "1/16", "1/8T", "1/16.", "1/8", "1/4T", "1/8.", "1/4", "1/2T", "1/4.", "1/2", "1T", "1/2.", "1", "2T", "1.", "2", "4"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 LFO2:Delay Time", offset: 1744, size: 1, min: 0, max: 101, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 LFO2:Fade time", offset: 1745, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 LFO2:Offset", offset: 1746, size: 1, min: 0, max: 2, labels: &["-", "0", "+"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 LFO2:Key trig", offset: 1747, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 WG:Waveform", offset: 1748, size: 2, min: 0, max: 108, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 Reserved", offset: 1750, size: 1, min: 0, max: 0, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 WG:Wave gain", offset: 1751, size: 1, min: 0, max: 5, labels: &["-18", "-12", "-6", "0", "+6", "+12[dB]"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 WG:Pitch coarse", offset: 1752, size: 1, min: -48, max: 48, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 WG:Pitch fine", offset: 1753, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 WG:Pitch random", offset: 1754, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 WG:Pitch KF", offset: 1755, size: 1, min: 0, max: 16, labels: &["-100", "-50", "-20", "-10", "-5", "0", "+5", "+10", "+20", "+50", "+98", "+99", "+100", "+101", "+102", "+150", "+200[%]"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 WG:Bender Sw", offset: 1756, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 WG:Atch bend sw", offset: 1757, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 WG:LFO1 depth", offset: 1758, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 WG:LFO2 depth", offset: 1759, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 WG:Lever sens", offset: 1760, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 WG:Atch msens", offset: 1761, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 Pitch Env Velocity Sens", offset: 1762, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 Pitch Env Time Velocity Sens", offset: 1763, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 Pitch Env Time Keyfollow", offset: 1764, size: 1, min: -10, max: 10, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 Pitch Env L0", offset: 1765, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 Pitch Env L1", offset: 1766, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 Pitch Env L2", offset: 1767, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 Pitch Env T1", offset: 1768, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 Pitch Env T2", offset: 1769, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 Pitch Env T3", offset: 1770, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 TVF:Filter mode", offset: 1771, size: 1, min: 0, max: 2, labels: &["LPF", "BPF", "HPF"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 TVF:Cutoff freq", offset: 1772, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 TVF:Resonance", offset: 1773, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 TVF:Cutoff KF", offset: 1774, size: 1, min: 0, max: 40, labels: &["-100", "-90", "-80", "-70", "-60", "-50", "-40", "-30", "-20", "-10", "0", "+5", "+10", "+15", "+20", "+25", "+30", "+35", "+40", "+45", "+50", "+55", "+60", "+65", "+70", "+75", "+80", "+85", "+90", "+95", "+100", "+105", "+110", "+115", "+120", "+125", "+130", "+135", "+140", "+145", "+150[%]"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 TVF:Atch sens", offset: 1775, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 TVF:LFO select", offset: 1776, size: 1, min: 0, max: 1, labels: &["LFO1", "LFO2"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 TVF:LFO depth", offset: 1777, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 TVF:ENV depth", offset: 1778, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 F-ENV:Velo sens", offset: 1779, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 F-ENV:T vsens", offset: 1780, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 F-ENV:Time KF", offset: 1781, size: 1, min: -10, max: 10, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 F-ENV:Level 1", offset: 1782, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 F-ENV:Level 2", offset: 1783, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 F-ENV:Sus Level", offset: 1784, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 F-ENV:Level 4", offset: 1785, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 F-ENV:Time 1", offset: 1786, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 F-ENV:Time 2", offset: 1787, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 F-ENV:Time 3", offset: 1788, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 F-ENV:Time 4", offset: 1789, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 TVA Bias Direction", offset: 1790, size: 1, min: 0, max: 2, labels: &["UP", "LOW", "U&L"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 TVA Bias Point", offset: 1791, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 TVA:Bias level", offset: 1792, size: 1, min: -10, max: 10, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 TVA:Level", offset: 1793, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 TVA After Touch Sens", offset: 1794, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 TVA:LFO select", offset: 1795, size: 1, min: 0, max: 1, labels: &["LFO1", "LFO2"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 TVA:LFO depth", offset: 1796, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 A-ENV:Velo sens", offset: 1797, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 A-ENV:T vsens", offset: 1798, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 A-ENV:Time KF", offset: 1799, size: 1, min: -10, max: 10, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 A-ENV:Level 1", offset: 1800, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 A-ENV:Level 2", offset: 1801, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 A-ENV:Sus level", offset: 1802, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 A-ENV:Time 1", offset: 1803, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 A-ENV:Time 2", offset: 1804, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 A-ENV:Time 3", offset: 1805, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL2 A-ENV:Time 4", offset: 1806, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 Layer Partial Switch", offset: 1808, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 Active Partial Switch", offset: 1809, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 Velocity Curve", offset: 1810, size: 1, min: 0, max: 3, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 Hold Control Switch", offset: 1811, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 LFO1:Waveform", offset: 1812, size: 1, min: 0, max: 4, labels: &["TRI", "SAW", "SQU", "S&H", "RND"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 LFO1:Rate sync", offset: 1813, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 LFO1:Rate", offset: 1814, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 LFO1:Rate nt", offset: 1815, size: 1, min: 0, max: 22, labels: &["1/64T", "1/64", "1/32T", "1/32", "1/16T", "1/32.", "1/16", "1/8T", "1/16.", "1/8", "1/4T", "1/8.", "1/4", "1/2T", "1/4.", "1/2", "1T", "1/2.", "1", "2T", "1.", "2", "4"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 LFO1:Delay time", offset: 1816, size: 1, min: 0, max: 101, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 LFO1:Fade time", offset: 1817, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 LFO1:Offset", offset: 1818, size: 1, min: 0, max: 2, labels: &["-", "0", "+"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 LFO1:Key trig", offset: 1819, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 LFO2:Waveform", offset: 1820, size: 1, min: 0, max: 4, labels: &["TRI", "SAW", "SQU", "S&H", "RND"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 LFO2:Rate Sync", offset: 1821, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 LFO2:Rate", offset: 1822, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 LFO2:Rate nt", offset: 1823, size: 1, min: 0, max: 22, labels: &["1/64T", "1/64", "1/32T", "1/32", "1/16T", "1/32.", "1/16", "1/8T", "1/16.", "1/8", "1/4T", "1/8.", "1/4", "1/2T", "1/4.", "1/2", "1T", "1/2.", "1", "2T", "1.", "2", "4"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 LFO2:Delay Time", offset: 1824, size: 1, min: 0, max: 101, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 LFO2:Fade time", offset: 1825, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 LFO2:Offset", offset: 1826, size: 1, min: 0, max: 2, labels: &["-", "0", "+"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 LFO2:Key trig", offset: 1827, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 WG:Waveform", offset: 1828, size: 2, min: 0, max: 108, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 Reserved", offset: 1830, size: 1, min: 0, max: 0, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 WG:Wave gain", offset: 1831, size: 1, min: 0, max: 5, labels: &["-18", "-12", "-6", "0", "+6", "+12[dB]"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 WG:Pitch coarse", offset: 1832, size: 1, min: -48, max: 48, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 WG:Pitch fine", offset: 1833, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 WG:Pitch random", offset: 1834, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 WG:Pitch KF", offset: 1835, size: 1, min: 0, max: 16, labels: &["-100", "-50", "-20", "-10", "-5", "0", "+5", "+10", "+20", "+50", "+98", "+99", "+100", "+101", "+102", "+150", "+200[%]"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 WG:Bender Sw", offset: 1836, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 WG:Atch bend sw", offset: 1837, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 WG:LFO1 depth", offset: 1838, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 WG:LFO2 depth", offset: 1839, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 WG:Lever sens", offset: 1840, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 WG:Atch msens", offset: 1841, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 Pitch Env Velocity Sens", offset: 1842, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 Pitch Env Time Velocity Sens", offset: 1843, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 Pitch Env Time Keyfollow", offset: 1844, size: 1, min: -10, max: 10, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 Pitch Env L0", offset: 1845, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 Pitch Env L1", offset: 1846, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 Pitch Env L2", offset: 1847, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 Pitch Env T1", offset: 1848, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 Pitch Env T2", offset: 1849, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 Pitch Env T3", offset: 1850, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 TVF:Filter mode", offset: 1851, size: 1, min: 0, max: 2, labels: &["LPF", "BPF", "HPF"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 TVF:Cutoff freq", offset: 1852, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 TVF:Resonance", offset: 1853, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 TVF:Cutoff KF", offset: 1854, size: 1, min: 0, max: 40, labels: &["-100", "-90", "-80", "-70", "-60", "-50", "-40", "-30", "-20", "-10", "0", "+5", "+10", "+15", "+20", "+25", "+30", "+35", "+40", "+45", "+50", "+55", "+60", "+65", "+70", "+75", "+80", "+85", "+90", "+95", "+100", "+105", "+110", "+115", "+120", "+125", "+130", "+135", "+140", "+145", "+150[%]"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 TVF:Atch sens", offset: 1855, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 TVF:LFO select", offset: 1856, size: 1, min: 0, max: 1, labels: &["LFO1", "LFO2"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 TVF:LFO depth", offset: 1857, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 TVF:ENV depth", offset: 1858, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 F-ENV:Velo sens", offset: 1859, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 F-ENV:T vsens", offset: 1860, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 F-ENV:Time KF", offset: 1861, size: 1, min: -10, max: 10, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 F-ENV:Level 1", offset: 1862, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 F-ENV:Level 2", offset: 1863, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 F-ENV:Sus Level", offset: 1864, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 F-ENV:Level 4", offset: 1865, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 F-ENV:Time 1", offset: 1866, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 F-ENV:Time 2", offset: 1867, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 F-ENV:Time 3", offset: 1868, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 F-ENV:Time 4", offset: 1869, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 TVA Bias Direction", offset: 1870, size: 1, min: 0, max: 2, labels: &["UP", "LOW", "U&L"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 TVA Bias Point", offset: 1871, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 TVA:Bias level", offset: 1872, size: 1, min: -10, max: 10, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 TVA:Level", offset: 1873, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 TVA After Touch Sens", offset: 1874, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 TVA:LFO select", offset: 1875, size: 1, min: 0, max: 1, labels: &["LFO1", "LFO2"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 TVA:LFO depth", offset: 1876, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 A-ENV:Velo sens", offset: 1877, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 A-ENV:T vsens", offset: 1878, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 A-ENV:Time KF", offset: 1879, size: 1, min: -10, max: 10, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 A-ENV:Level 1", offset: 1880, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 A-ENV:Level 2", offset: 1881, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 A-ENV:Sus level", offset: 1882, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 A-ENV:Time 1", offset: 1883, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 A-ENV:Time 2", offset: 1884, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 A-ENV:Time 3", offset: 1885, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL3 A-ENV:Time 4", offset: 1886, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 Layer Partial Switch", offset: 1888, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 Active Partial Switch", offset: 1889, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 Velocity Curve", offset: 1890, size: 1, min: 0, max: 3, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 Hold Control Switch", offset: 1891, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 LFO1:Waveform", offset: 1892, size: 1, min: 0, max: 4, labels: &["TRI", "SAW", "SQU", "S&H", "RND"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 LFO1:Rate sync", offset: 1893, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 LFO1:Rate", offset: 1894, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 LFO1:Rate nt", offset: 1895, size: 1, min: 0, max: 22, labels: &["1/64T", "1/64", "1/32T", "1/32", "1/16T", "1/32.", "1/16", "1/8T", "1/16.", "1/8", "1/4T", "1/8.", "1/4", "1/2T", "1/4.", "1/2", "1T", "1/2.", "1", "2T", "1.", "2", "4"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 LFO1:Delay time", offset: 1896, size: 1, min: 0, max: 101, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 LFO1:Fade time", offset: 1897, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 LFO1:Offset", offset: 1898, size: 1, min: 0, max: 2, labels: &["-", "0", "+"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 LFO1:Key trig", offset: 1899, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 LFO2:Waveform", offset: 1900, size: 1, min: 0, max: 4, labels: &["TRI", "SAW", "SQU", "S&H", "RND"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 LFO2:Rate Sync", offset: 1901, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 LFO2:Rate", offset: 1902, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 LFO2:Rate nt", offset: 1903, size: 1, min: 0, max: 22, labels: &["1/64T", "1/64", "1/32T", "1/32", "1/16T", "1/32.", "1/16", "1/8T", "1/16.", "1/8", "1/4T", "1/8.", "1/4", "1/2T", "1/4.", "1/2", "1T", "1/2.", "1", "2T", "1.", "2", "4"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 LFO2:Delay Time", offset: 1904, size: 1, min: 0, max: 101, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 LFO2:Fade time", offset: 1905, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 LFO2:Offset", offset: 1906, size: 1, min: 0, max: 2, labels: &["-", "0", "+"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 LFO2:Key trig", offset: 1907, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 WG:Waveform", offset: 1908, size: 2, min: 0, max: 108, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 Reserved", offset: 1910, size: 1, min: 0, max: 0, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 WG:Wave gain", offset: 1911, size: 1, min: 0, max: 5, labels: &["-18", "-12", "-6", "0", "+6", "+12[dB]"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 WG:Pitch coarse", offset: 1912, size: 1, min: -48, max: 48, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 WG:Pitch fine", offset: 1913, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 WG:Pitch random", offset: 1914, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 WG:Pitch KF", offset: 1915, size: 1, min: 0, max: 16, labels: &["-100", "-50", "-20", "-10", "-5", "0", "+5", "+10", "+20", "+50", "+98", "+99", "+100", "+101", "+102", "+150", "+200[%]"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 WG:Bender Sw", offset: 1916, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 WG:Atch bend sw", offset: 1917, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 WG:LFO1 depth", offset: 1918, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 WG:LFO2 depth", offset: 1919, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 WG:Lever sens", offset: 1920, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 WG:Atch msens", offset: 1921, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 Pitch Env Velocity Sens", offset: 1922, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 Pitch Env Time Velocity Sens", offset: 1923, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 Pitch Env Time Keyfollow", offset: 1924, size: 1, min: -10, max: 10, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 Pitch Env L0", offset: 1925, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 Pitch Env L1", offset: 1926, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 Pitch Env L2", offset: 1927, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 Pitch Env T1", offset: 1928, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 Pitch Env T2", offset: 1929, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 Pitch Env T3", offset: 1930, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 TVF:Filter mode", offset: 1931, size: 1, min: 0, max: 2, labels: &["LPF", "BPF", "HPF"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 TVF:Cutoff freq", offset: 1932, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 TVF:Resonance", offset: 1933, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 TVF:Cutoff KF", offset: 1934, size: 1, min: 0, max: 40, labels: &["-100", "-90", "-80", "-70", "-60", "-50", "-40", "-30", "-20", "-10", "0", "+5", "+10", "+15", "+20", "+25", "+30", "+35", "+40", "+45", "+50", "+55", "+60", "+65", "+70", "+75", "+80", "+85", "+90", "+95", "+100", "+105", "+110", "+115", "+120", "+125", "+130", "+135", "+140", "+145", "+150[%]"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 TVF:Atch sens", offset: 1935, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 TVF:LFO select", offset: 1936, size: 1, min: 0, max: 1, labels: &["LFO1", "LFO2"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 TVF:LFO depth", offset: 1937, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 TVF:ENV depth", offset: 1938, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 F-ENV:Velo sens", offset: 1939, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 F-ENV:T vsens", offset: 1940, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 F-ENV:Time KF", offset: 1941, size: 1, min: -10, max: 10, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 F-ENV:Level 1", offset: 1942, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 F-ENV:Level 2", offset: 1943, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 F-ENV:Sus Level", offset: 1944, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 F-ENV:Level 4", offset: 1945, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 F-ENV:Time 1", offset: 1946, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 F-ENV:Time 2", offset: 1947, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 F-ENV:Time 3", offset: 1948, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 F-ENV:Time 4", offset: 1949, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 TVA Bias Direction", offset: 1950, size: 1, min: 0, max: 2, labels: &["UP", "LOW", "U&L"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 TVA Bias Point", offset: 1951, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 TVA:Bias level", offset: 1952, size: 1, min: -10, max: 10, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 TVA:Level", offset: 1953, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 TVA After Touch Sens", offset: 1954, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 TVA:LFO select", offset: 1955, size: 1, min: 0, max: 1, labels: &["LFO1", "LFO2"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 TVA:LFO depth", offset: 1956, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 A-ENV:Velo sens", offset: 1957, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 A-ENV:T vsens", offset: 1958, size: 1, min: -50, max: 50, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 A-ENV:Time KF", offset: 1959, size: 1, min: -10, max: 10, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 A-ENV:Level 1", offset: 1960, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 A-ENV:Level 2", offset: 1961, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 A-ENV:Sus level", offset: 1962, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 A-ENV:Time 1", offset: 1963, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 A-ENV:Time 2", offset: 1964, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 A-ENV:Time 3", offset: 1965, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL4 A-ENV:Time 4", offset: 1966, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Sequence", offset: 1968, size: 1, min: 0, max: 5, labels: &["CHO-DLY-REV", "CHO-REV-DLY", "DLY-CHO-REV", "DLY-REV-CHO", "REV-CHO-DLY", "REV-DLY-CHO"], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB DL Switch", offset: 1969, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Delay Center (sync sw)", offset: 1970, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Delay Center (time)", offset: 1971, size: 1, min: 0, max: 125, labels: &["0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "1.0", "1.1", "1.2", "1.3", "1.4", "1.5", "1.6", "1.7", "1.8", "1.9", "2.0", "2.1", "2.2", "2.3", "2.4", "2.5", "2.6", "2.7", "2.8", "2.9", "3.0", "3.1", "3.2", "3.3", "3.4", "3.5", "3.6", "3.7", "3.8", "3.9", "4.0", "4.1", "4.2", "4.3", "4.4", "4.5", "4.6", "4.7", "4.8", "4.9", "5.0", "5.5", "6.0", "6.5", "7.0", "7.5", "8.0", "8.5", "9.0", "9.5", "10.0", "11.0", "12.0", "13.0", "14.0", "15.0", "16.0", "17.0", "18.0", "19.0", "20.0", "21.0", "22.0", "23.0", "24.0", "25.0", "26.0", "27.0", "28.0", "29.0", "30.0", "31.0", "32.0", "33.0", "34.0", "35.0", "36.0", "37.0", "38.0", "39.0", "40.0", "50.0", "60.0", "70.0", "80.0", "90.0", "100.0", "110.0", "120.0", "130.0", "140.0", "150.0", "160.0", "170.0", "180.0", "190.0", "200.0", "220.0", "240.0", "260.0", "280.0", "300.0", "320.0", "340.0", "360.0", "380.0", "400.0", "420.0", "440.0", "460.0", "480.0", "500.0", "520.0", "540.0", "560.0", "580.0", "600.0 [msec]"], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Delay Center (note)", offset: 1972, size: 1, min: 0, max: 21, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Delay Left (sync sw)", offset: 1973, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Delay Left (time)", offset: 1974, size: 1, min: 0, max: 125, labels: &["0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "1.0", "1.1", "1.2", "1.3", "1.4", "1.5", "1.6", "1.7", "1.8", "1.9", "2.0", "2.1", "2.2", "2.3", "2.4", "2.5", "2.6", "2.7", "2.8", "2.9", "3.0", "3.1", "3.2", "3.3", "3.4", "3.5", "3.6", "3.7", "3.8", "3.9", "4.0", "4.1", "4.2", "4.3", "4.4", "4.5", "4.6", "4.7", "4.8", "4.9", "5.0", "5.5", "6.0", "6.5", "7.0", "7.5", "8.0", "8.5", "9.0", "9.5", "10.0", "11.0", "12.0", "13.0", "14.0", "15.0", "16.0", "17.0", "18.0", "19.0", "20.0", "21.0", "22.0", "23.0", "24.0", "25.0", "26.0", "27.0", "28.0", "29.0", "30.0", "31.0", "32.0", "33.0", "34.0", "35.0", "36.0", "37.0", "38.0", "39.0", "40.0", "50.0", "60.0", "70.0", "80.0", "90.0", "100.0", "110.0", "120.0", "130.0", "140.0", "150.0", "160.0", "170.0", "180.0", "190.0", "200.0", "220.0", "240.0", "260.0", "280.0", "300.0", "320.0", "340.0", "360.0", "380.0", "400.0", "420.0", "440.0", "460.0", "480.0", "500.0", "520.0", "540.0", "560.0", "580.0", "600.0 [msec]"], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Delay Left (note)", offset: 1975, size: 1, min: 0, max: 21, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Delay Right (sync sw)", offset: 1976, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Delay Right (time)", offset: 1977, size: 1, min: 0, max: 125, labels: &["0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "1.0", "1.1", "1.2", "1.3", "1.4", "1.5", "1.6", "1.7", "1.8", "1.9", "2.0", "2.1", "2.2", "2.3", "2.4", "2.5", "2.6", "2.7", "2.8", "2.9", "3.0", "3.1", "3.2", "3.3", "3.4", "3.5", "3.6", "3.7", "3.8", "3.9", "4.0", "4.1", "4.2", "4.3", "4.4", "4.5", "4.6", "4.7", "4.8", "4.9", "5.0", "5.5", "6.0", "6.5", "7.0", "7.5", "8.0", "8.5", "9.0", "9.5", "10.0", "11.0", "12.0", "13.0", "14.0", "15.0", "16.0", "17.0", "18.0", "19.0", "20.0", "21.0", "22.0", "23.0", "24.0", "25.0", "26.0", "27.0", "28.0", "29.0", "30.0", "31.0", "32.0", "33.0", "34.0", "35.0", "36.0", "37.0", "38.0", "39.0", "40.0", "50.0", "60.0", "70.0", "80.0", "90.0", "100.0", "110.0", "120.0", "130.0", "140.0", "150.0", "160.0", "170.0", "180.0", "190.0", "200.0", "220.0", "240.0", "260.0", "280.0", "300.0", "320.0", "340.0", "360.0", "380.0", "400.0", "420.0", "440.0", "460.0", "480.0", "500.0", "520.0", "540.0", "560.0", "580.0", "600.0 [msec]"], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Delay Right (note)", offset: 1978, size: 1, min: 0, max: 21, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Center Level", offset: 1979, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Left Level", offset: 1980, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Right Level", offset: 1981, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Delay Feedback", offset: 1982, size: 1, min: 0, max: 98, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Chorus Sw", offset: 1983, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Chorus Rate", offset: 1984, size: 1, min: 0, max: 99, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Chorus Depth", offset: 1985, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Chorus Delay", offset: 1986, size: 1, min: 0, max: 99, labels: &["0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "1.0", "1.1", "1.2", "1.3", "1.4", "1.5", "1.6", "1.7", "1.8", "1.9", "2.0", "2.1", "2.2", "2.3", "2.4", "2.5", "2.6", "2.7", "2.8", "2.9", "3.0", "3.1", "3.2", "3.3", "3.4", "3.5", "3.6", "3.7", "3.8", "3.9", "4.0", "4.1", "4.2", "4.3", "4.4", "4.5", "4.6", "4.7", "4.8", "4.9", "5.0", "5.5", "6.0", "6.5", "7.0", "7.5", "8.0", "8.5", "9.0", "9.5", "10.0", "11.0", "12.0", "13.0", "14.0", "15.0", "16.0", "17.0", "18.0", "19.0", "20.0", "21.0", "22.0", "23.0", "24.0", "25.0", "26.0", "27.0", "28.0", "29.0", "30.0", "31.0", "32.0", "33.0", "34.0", "35.0", "36.0", "37.0", "38.0", "39.0", "40.0", "41.0", "42.0", "43.0", "44.0", "45.0", "46.0", "47.0", "48.0", "49.0", "50.0 [msec]"], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Chorus Feedback", offset: 1987, size: 1, min: 0, max: 98, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Chorus Level", offset: 1988, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Reverb Sw", offset: 1989, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Reverb Type", offset: 1990, size: 1, min: 0, max: 9, labels: &["ROOM1", "ROOM2", "HALL1", "HALL2", "HALL3", "HALL4", "GATE", "REVERSE", "FLYING1", "FLYING2"], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Reverb Pre Delay", offset: 1991, size: 1, min: 0, max: 120, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Reverb ER Lev", offset: 1992, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Reverb HF Damp", offset: 1993, size: 1, min: 0, max: 16, labels: &["500Hz", "630Hz", "800Hz", "1kHz", "1.25kHz", "1.6kHz", "2kHz", "2.5kHz", "3.15kHz", "4kHz", "5kHz", "6.3kHz", "8kHz", "10kHz", "12.5kHz", "16kHz", "BYPASS"], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Reverb Time", offset: 1994, size: 1, min: 0, max: 99, labels: &["0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "1.0", "1.1", "1.2", "1.3", "1.4", "1.5", "1.6", "1.7", "1.8", "1.9", "2.0", "2.1", "2.2", "2.3", "2.4", "2.5", "2.6", "2.7", "2.8", "2.9", "3.0", "3.1", "3.2", "3.3", "3.4", "3.5", "3.6", "3.7", "3.8", "3.9", "4.0", "4.1", "4.2", "4.3", "4.4", "4.5", "4.6", "4.7", "4.8", "4.9", "5.0", "5.1", "5.2", "5.3", "5.4", "5.5", "5.6", "5.7", "5.8", "5.9", "6.0", "6.1", "6.2", "6.3", "6.4", "6.5", "6.6", "6.7", "6.8", "6.9", "7.0", "7.1", "7.2", "7.3", "7.4", "7.5", "7.6", "7.7", "7.8", "7.9", "8.0", "8.5", "9.0", "9.5", "10.0", "10.5", "11.0", "11.5", "12.0", "12.5", "13.0", "13.5", "14.0", "14.5", "15.0", "15.5", "16.0", "17.0", "18.0", "19.0", "20.0 [sec]"], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Reverb Level", offset: 1995, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Balance", offset: 1996, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "FxB Level", offset: 1997, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM Level", offset: 2000, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM Key Range A Low", offset: 2001, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM Key Range A High", offset: 2002, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM Key Range B Low", offset: 2003, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM Key Range B High", offset: 2004, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM Key Range C Low", offset: 2005, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM Key Range C High", offset: 2006, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM Key Range D Low", offset: 2007, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM Key Range D High", offset: 2008, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM Bender Range Down", offset: 2009, size: 1, min: 0, max: 48, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM Bender Range Up", offset: 2010, size: 1, min: 0, max: 12, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM After Touch Bend Sens", offset: 2011, size: 1, min: 0, max: 26, labels: &["-36", "-24", "-12", "-11", "-10", "-9", "-8", "-7", "-6", "-5", "-4", "-3", "-2", "-1", "0", "+1", "+2", "+3", "+4", "+5", "+6", "+7", "+8", "+9", "+10", "+11", "+12"], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM Solo Switch", offset: 2012, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM Solo Legato Switch", offset: 2013, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM Portamento Switch", offset: 2014, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM Portamento Mode", offset: 2015, size: 1, min: 0, max: 1, labels: &["NORMAL", "LEGATO"], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM Portamento Time", offset: 2016, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM EQ Mid Q", offset: 2017, size: 1, min: 5, max: 160, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM EQ Low Frequency", offset: 2018, size: 2, min: 20, max: 16000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM EQ Mid Frequency", offset: 2020, size: 2, min: 20, max: 16000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM EQ High Frequency", offset: 2022, size: 2, min: 20, max: 16000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM EQ Low Gain", offset: 2024, size: 2, min: -240, max: 240, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM EQ Mid Gain", offset: 2026, size: 2, min: -240, max: 240, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM EQ High Gain", offset: 2028, size: 2, min: -240, max: 240, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM EQ Switch", offset: 2030, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "COM Unison Switch", offset: 2031, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
];

#[rustfmt::skip]
pub static WT: &[MdlParam] = &[
    MdlParam { id: "OSC Waveform Type", offset: 1648, size: 1, min: 0, max: 1, labels: &["DRAWING", "TABLE"], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Morph Rough", offset: 1649, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 1", offset: 1652, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 2", offset: 1653, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 3", offset: 1654, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 4", offset: 1655, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 5", offset: 1656, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 6", offset: 1657, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 7", offset: 1658, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 8", offset: 1659, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 9", offset: 1660, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 10", offset: 1661, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 11", offset: 1662, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 12", offset: 1663, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 13", offset: 1664, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 14", offset: 1665, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 15", offset: 1666, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 16", offset: 1667, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 17", offset: 1668, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 18", offset: 1669, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 19", offset: 1670, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 20", offset: 1671, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 21", offset: 1672, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 22", offset: 1673, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 23", offset: 1674, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 24", offset: 1675, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 25", offset: 1676, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 26", offset: 1677, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 27", offset: 1678, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 28", offset: 1679, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 29", offset: 1680, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 30", offset: 1681, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 31", offset: 1682, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Data 32", offset: 1683, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Group", offset: 1684, size: 1, min: 0, max: 1, labels: &["EXP", "USER"], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Position", offset: 1685, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Layer Mix", offset: 1686, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Layer Mix Curve", offset: 1687, size: 1, min: 0, max: 1, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Layer1 Group ID", offset: 1688, size: 2, min: 0, max: 16383, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Layer1 Number", offset: 1690, size: 2, min: 0, max: 16383, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Layer2 Group ID", offset: 1692, size: 2, min: 0, max: 16383, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Wave Layer2 Number", offset: 1694, size: 2, min: 0, max: 16383, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Position LFO Depth", offset: 1696, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Position Env Depth", offset: 1697, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Layer Mix LFO Depth", offset: 1698, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Layer Mix Env Depth", offset: 1699, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Position Random Sens", offset: 1700, size: 1, min: 0, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Layer Mix Random Sens", offset: 1701, size: 1, min: 0, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Layer2 Octave Down", offset: 1702, size: 1, min: 0, max: 2, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-B Table Number", offset: 1704, size: 1, min: 0, max: 14, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-A-B Link", offset: 1705, size: 1, min: 0, max: 1, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-A Limiter", offset: 1706, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-A-X", offset: 1708, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-B Depth", offset: 1709, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-A-Y", offset: 1710, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-B Drive", offset: 1711, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-A LFO Depth", offset: 1712, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-A Env Depth", offset: 1713, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-A-X Random Sens", offset: 1714, size: 1, min: 0, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-A-Y Random Sens", offset: 1715, size: 1, min: 0, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-B LFO Depth", offset: 1716, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-B Env Depth", offset: 1717, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-B Depth Random Sens", offset: 1718, size: 1, min: 0, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-B Drive Random Sens", offset: 1719, size: 1, min: 0, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Osc Env Attack Time", offset: 1720, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Osc Env Decay Time", offset: 1722, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Spread Switch", offset: 1724, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Spread Depth", offset: 1725, size: 1, min: 0, max: 20, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Spread Timing Tempo Sync", offset: 1726, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC DelayTime (note)", offset: 1727, size: 1, min: 0, max: 21, labels: &["1/64T", "1/64", "1/32T", "1/32", "1/16T", "1/32.", "1/16", "1/8T", "1/16.", "1/8", "1/4T", "1/8.", "1/4", "1/2T", "1/4.", "1/2", "1T", "1/2.", "1", "2T", "1.", "2"], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC DelayTime", offset: 1728, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC LFO Pitch Depth", offset: 1730, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-A-X Velocity Sens", offset: 1732, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-A-Y Velocity Sens", offset: 1733, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-B Depth Velocity Sens", offset: 1734, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Mod-B Drive Velocity Sens", offset: 1735, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC WT Pos Velocity Sens", offset: 1736, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC WT PosY Velocity Sens", offset: 1737, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Draw Tool Form", offset: 1740, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Draw Tool Mod", offset: 1741, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Draw Tool Sync", offset: 1742, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Draw Tool Width", offset: 1743, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "OSC Draw Tool Exec", offset: 1744, size: 1, min: 0, max: 1, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Mod Pitch Switch", offset: 1756, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Mod Mod-A Switch", offset: 1757, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Mod MOD-B Switch", offset: 1758, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Mod Position Switch", offset: 1759, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Data 1", offset: 1760, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Data 2", offset: 1761, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Data 3", offset: 1762, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Data 4", offset: 1763, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Data 5", offset: 1764, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Data 6", offset: 1765, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Data 7", offset: 1766, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Data 8", offset: 1767, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Data 9", offset: 1768, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Data 10", offset: 1769, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Data 11", offset: 1770, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Data 12", offset: 1771, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Data 13", offset: 1772, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Data 14", offset: 1773, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Data 15", offset: 1774, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Data 16", offset: 1775, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Curve 1", offset: 1776, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Curve 2", offset: 1777, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Curve 3", offset: 1778, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Curve 4", offset: 1779, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Curve 5", offset: 1780, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Curve 6", offset: 1781, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Curve 7", offset: 1782, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Curve 8", offset: 1783, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Curve 9", offset: 1784, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Curve 10", offset: 1785, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Curve 11", offset: 1786, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Curve 12", offset: 1787, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Curve 13", offset: 1788, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Curve 14", offset: 1789, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Curve 15", offset: 1790, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Osc Step Curve 16", offset: 1791, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Scale Switch", offset: 1792, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Scale Table", offset: 1793, size: 1, min: 0, max: 18, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Mod Filter Switch", offset: 1794, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Mod Amp Switch", offset: 1795, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Data 1", offset: 1796, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Data 2", offset: 1797, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Data 3", offset: 1798, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Data 4", offset: 1799, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Data 5", offset: 1800, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Data 6", offset: 1801, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Data 7", offset: 1802, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Data 8", offset: 1803, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Data 9", offset: 1804, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Data 10", offset: 1805, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Data 11", offset: 1806, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Data 12", offset: 1807, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Data 13", offset: 1808, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Data 14", offset: 1809, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Data 15", offset: 1810, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Data 16", offset: 1811, size: 1, min: -72, max: 72, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Curve 1", offset: 1812, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Curve 2", offset: 1813, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Curve 3", offset: 1814, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Curve 4", offset: 1815, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Curve 5", offset: 1816, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Curve 6", offset: 1817, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Curve 7", offset: 1818, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Curve 8", offset: 1819, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Curve 9", offset: 1820, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Curve 10", offset: 1821, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Curve 11", offset: 1822, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Curve 12", offset: 1823, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Curve 13", offset: 1824, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Curve 14", offset: 1825, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Curve 15", offset: 1826, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Filter-Amp Step Curve 16", offset: 1827, size: 1, min: 0, max: 36, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Switch 1", offset: 1828, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Switch 2", offset: 1829, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Switch 3", offset: 1830, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Switch 4", offset: 1831, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Switch 5", offset: 1832, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Switch 6", offset: 1833, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Switch 7", offset: 1834, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Switch 8", offset: 1835, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Switch 9", offset: 1836, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Switch 10", offset: 1837, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Switch 11", offset: 1838, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Switch 12", offset: 1839, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Switch 13", offset: 1840, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Switch 14", offset: 1841, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Switch 15", offset: 1842, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Switch 16", offset: 1843, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Kick Switch 1", offset: 1844, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Kick Switch 2", offset: 1845, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Kick Switch 3", offset: 1846, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Kick Switch 4", offset: 1847, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Kick Switch 5", offset: 1848, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Kick Switch 6", offset: 1849, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Kick Switch 7", offset: 1850, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Kick Switch 8", offset: 1851, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Kick Switch 9", offset: 1852, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Kick Switch 10", offset: 1853, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Kick Switch 11", offset: 1854, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Kick Switch 12", offset: 1855, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Kick Switch 13", offset: 1856, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Kick Switch 14", offset: 1857, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Kick Switch 15", offset: 1858, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Kick Switch 16", offset: 1859, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Size", offset: 1860, size: 1, min: 0, max: 15, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Envelope Trigger", offset: 1861, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Rate (note)", offset: 1862, size: 1, min: 0, max: 22, labels: &["1/64T", "1/64", "1/32T", "1/32", "1/16T", "1/32.", "1/16", "1/8T", "1/16.", "1/8", "1/4T", "1/8.", "1/4", "1/2T", "1/4.", "1/2", "1T", "1/2.", "1", "2T", "1.", "2", "4"], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Mod Mod-A Depth", offset: 1864, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Mod-Mod B Depth", offset: 1865, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Mod Position Depth", offset: 1866, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "MOD Step Mod Filter Depth", offset: 1867, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Mono/Poly", offset: 1876, size: 1, min: 0, max: 1, labels: &["MONO", "POLY"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Octave Shift", offset: 1877, size: 1, min: -3, max: 3, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Bend Range Up", offset: 1878, size: 1, min: 0, max: 48, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Bend Range Down", offset: 1879, size: 1, min: 0, max: 48, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Envelope Select", offset: 1880, size: 1, min: 0, max: 2, labels: &["FILTER", "AMP", "FILTER+AMP"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Portamento Switch", offset: 1881, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Portamento Curve Type", offset: 1882, size: 1, min: 0, max: 1, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Portamento Mode", offset: 1883, size: 1, min: 0, max: 1, labels: &["NORMAL", "LEGATO"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Portamento Time", offset: 1884, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Filter Type", offset: 1888, size: 1, min: 0, max: 1, labels: &["TVF", "VCF"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Filter Slope", offset: 1889, size: 1, min: 0, max: 1, labels: &["-12", "-24 [dB/Oct]"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL HPF Cutoff", offset: 1890, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL TVF Filter Type", offset: 1892, size: 1, min: 0, max: 3, labels: &["LPF", "BPF", "HPF", "PKG"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL VCF Type", offset: 1893, size: 1, min: 0, max: 3, labels: &["1", "2", "3", "4"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Cutoff Frequency", offset: 1894, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Resonance", offset: 1896, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Cutoff Keyfollow", offset: 1898, size: 2, min: -200, max: 200, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Filter Env Depth", offset: 1900, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL LFO Filter Depth", offset: 1901, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Level", offset: 1902, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL LFO Amp Depth", offset: 1903, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Filter Env Attack Time", offset: 1904, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Filter Env Decay Time", offset: 1906, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Filter Env Sustain Level", offset: 1908, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Filter Env Release Time", offset: 1910, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Amp Env Attack Time", offset: 1912, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Amp Env Decay Time", offset: 1914, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Amp Env Sustain Level", offset: 1916, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Amp Env Release Time", offset: 1918, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL LFO Waveform", offset: 1920, size: 1, min: 0, max: 5, labels: &["SIN", "TRI", "SAW-UP", "SAW-DW", "SQR", "RND"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL LFO Rate Sync", offset: 1921, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Rate (note)", offset: 1922, size: 1, min: 0, max: 22, labels: &["1/64T", "1/64", "1/32T", "1/32", "1/16T", "1/32.", "1/16", "1/8T", "1/16.", "1/8", "1/4T", "1/8.", "1/4", "1/2T", "1/4.", "1/2", "1T", "1/2.", "1", "2T", "1.", "2", "4"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL LFO Rate", offset: 1924, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL LFO Delay Time", offset: 1926, size: 2, min: 0, max: 1023, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Mod Destination", offset: 1928, size: 1, min: 0, max: 8, labels: &["PIT-LFO", "FILTER-LFO", "AMP-LFO", "MOD-A", "MOD-B", "MOD-AB", "WT-POS", "MIX", "WT-POS-MIX"], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Mod Pit LFO Depth", offset: 1929, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Mod Filter LFO Depth", offset: 1930, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Mod Amp Depth", offset: 1931, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Mod Mod-A Depth", offset: 1932, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Mod Mod-B Depth", offset: 1933, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Mod Mod-A+B Depth", offset: 1934, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Mod WT Pos Depth", offset: 1935, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Mod Mix/Rough Depth", offset: 1936, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Mod WT Pos + MIX Depth", offset: 1937, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Velocity Curve", offset: 1938, size: 1, min: 0, max: 7, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Level Velocity Sens", offset: 1939, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Cutoff Velocity Sens", offset: 1940, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL Resonance Velocity Sens", offset: 1941, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL TVF Env Velocity Sens", offset: 1942, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL TVF Env T1 Velocity Sens", offset: 1943, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL TVF Env T4 Velocity Sens", offset: 1944, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL TVA Env Time1 Velocity Sens", offset: 1945, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL TVA Env Time4 Velocity Sens", offset: 1946, size: 1, min: -100, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "PTL (fw-only param)", offset: 1947, size: 1, min: 0, max: 1, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "EQ EQ Low Gain", offset: 1964, size: 2, min: -240, max: 240, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "EQ EQ Mid Gain", offset: 1966, size: 2, min: -240, max: 240, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "EQ EQ High Gain", offset: 1968, size: 2, min: -240, max: 240, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "EQ EQ Low Frequency", offset: 1970, size: 2, min: 20, max: 16000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "EQ EQ Mid Frequency", offset: 1972, size: 2, min: 20, max: 16000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "EQ EQ High Frequency", offset: 1974, size: 2, min: 20, max: 16000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "EQ EQ Mid Q", offset: 1976, size: 1, min: 5, max: 160, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "EQ EQ Switch", offset: 1977, size: 1, min: 0, max: 1, labels: &["OFF", "ON"], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 0", offset: 1980, size: 1, min: 0, max: 93, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 1", offset: 1981, size: 1, min: 0, max: 1, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 2", offset: 1982, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 3", offset: 1983, size: 1, min: 0, max: 127, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 4", offset: 1984, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 5", offset: 1985, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 6", offset: 1986, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 7", offset: 1987, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 8", offset: 1988, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 9", offset: 1989, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 10", offset: 1990, size: 1, min: 0, max: 100, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 11", offset: 1991, size: 1, min: -63, max: 63, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 12", offset: 1992, size: 1, min: 0, max: 11, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 13", offset: 1993, size: 1, min: 0, max: 11, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 14", offset: 1994, size: 1, min: 0, max: 11, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 15", offset: 1995, size: 1, min: 0, max: 11, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 16", offset: 1996, size: 2, min: -20000, max: 20000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 17", offset: 1998, size: 2, min: -20000, max: 20000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 18", offset: 2000, size: 2, min: -20000, max: 20000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 19", offset: 2002, size: 2, min: -20000, max: 20000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 20", offset: 2004, size: 2, min: -20000, max: 20000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 21", offset: 2006, size: 2, min: -20000, max: 20000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 22", offset: 2008, size: 2, min: -20000, max: 20000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 23", offset: 2010, size: 2, min: -20000, max: 20000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 24", offset: 2012, size: 2, min: -20000, max: 20000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 25", offset: 2014, size: 2, min: -20000, max: 20000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 26", offset: 2016, size: 2, min: -20000, max: 20000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 27", offset: 2018, size: 2, min: -20000, max: 20000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 28", offset: 2020, size: 2, min: -20000, max: 20000, labels: &[], mirror: MdlMirror::Unproven },
    MdlParam { id: "CTRL Ctrl 29", offset: 2022, size: 2, min: -20000, max: 20000, labels: &[], mirror: MdlMirror::Unproven },
];

/// Analog fields that hold one value in every recorded JP-8 tone, as (field index, value): a
/// mirror that reads one is proven only at that value.
#[rustfmt::skip]
pub static JP8_PINNED: &[(usize, i32)] = &[(4, 0), (5, 0), (15, 0), (24, 0), (36, 3), (44, 1), (50, 0), (57, 0), (82, 0)];

/// Curve used by LFO_RATE, LFO_RATE_EXP.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static LFO_RATE_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 1023,
    x: &[0, 170, 341, 511, 682, 852, 1023, 0],
    y: &[154, 236, 390, 550, 697, 841, 913],
    a: &[0.172629997, -0.00313000008, -0.068847999, 0.0172780007, -0.00162600004, 0.200183004, 0.0],
    b: &[0.0358729996, 0.0402299985, 0.0754700005, -0.0365790017, -0.00894100033, -0.613995016, 0.0],
    c: &[0.273849994, 0.863484979, 0.934554994, 0.87895, 0.857626975, 0.834864974, 0.207423002],
};

/// Curve used by LFO_DELAY.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static LFO_DELAY_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 1023,
    x: &[0, 170, 341, 511, 682, 852, 1023, 0],
    y: &[0, 7, 14, 70, 236, 507, 621],
    a: &[0.000119999997, 0.000349000009, -0.0854649991, -0.183607996, -0.740661979, 0.0902229995, 0.0],
    b: &[-0.000239999994, -0.000589000003, 0.373833001, 0.622053027, 1.109182, -0.645532012, 0.0],
    c: &[0.0412969999, 0.0411759987, 0.0410440005, 0.532315016, 1.22559798, 1.22197497, 0.201581001],
};

/// Curve used by LFO_FADE.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static LFO_FADE_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 1023,
    x: &[0, 170, 341, 511, 682, 852, 1023, 0],
    y: &[0, 30, 78, 149, 419, 615, 735],
    a: &[-0.00699099991, -0.0479810014, 0.156835005, -1.18560505, -0.0995879993, 0.00614400022, 0.0],
    b: &[0.0589540005, 0.107240997, -0.0311689992, 2.06440401, -0.0196119994, -0.238543004, 0.0],
    c: &[0.124508001, 0.221441999, 0.291981012, 0.700149, 1.27214098, 0.934153974, 0.475499004],
};

/// Curve used by OSC1_MOD.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static OSC1_MOD_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 100,
    x: &[0, 16, 33, 50, 66, 83, 100, 0],
    y: &[0, 11, 21, 28, 35, 45, 51],
    a: &[0.0130939996, -0.0759079978, 0.0502119996, 0.00300600007, -0.227751002, 0.0257699993, 0.0],
    b: &[-0.0612219982, 0.011678, -0.0865439996, 0.00884999987, 0.36362499, -0.169187993, 0.0],
    c: &[0.735628009, 0.652464986, 0.448096991, 0.42564401, 0.452360988, 0.496358991, 0.235293999],
};

/// Curve used by OSC1_PENV.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static OSC1_PENV_CURVE: MdlCurve = MdlCurve {
    signed: true,
    lo: 0,
    hi: 100,
    x: &[0, 16, 33, 50, 66, 83, 100, 0],
    y: &[0, 25, 43, 56, 64, 72, 76],
    a: &[-0.0701609999, -0.003486, -0.139319003, 0.00849299971, -0.0123030003, 0.0915030017, 0.0],
    b: &[-0.174045995, -0.185822994, 0.0379960015, -0.0325569995, -0.00153799995, -0.300653994, 0.0],
    c: &[1.80670702, 1.24813294, 0.866029024, 0.524064004, 0.484429002, 0.444444001, 0.117647],
};

/// Curve used by PW_C.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static PW_C_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 127,
    x: &[0, 21, 42, 63, 84, 105, 127, 0],
    y: &[65, 67, 71, 78, 85, 91, 94],
    a: &[-0.00952399988, 0.0857139975, 0.0, 0.0, 0.0121379998, 0.0374570005, 0.0],
    b: &[0.057142999, -0.0285710003, -0.0, -0.0, -0.0597570017, -0.151326999, 0.0],
    c: &[0.0476190001, 0.133332998, 0.333332986, 0.333332986, 0.333332986, 0.250232995, 0.0599509999],
};

/// Curve used by PW_D.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static PW_D_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 127,
    x: &[0, 21, 42, 63, 84, 105, 127, 0],
    y: &[0, -1, -6, -13, -20, -27, -30],
    a: &[-0.0317460001, -0.0317460001, -0.0, -0.0, -0.0, -0.0961939991, 0.0],
    b: &[-0.0634920001, -0.0317460001, 0.0, 0.0, 0.0, 0.293164015, 0.0],
    c: &[0.0476190001, -0.174603, -0.333332986, -0.333332986, -0.333332986, -0.333332986, -0.035588],
};

/// Curve used by PW_A.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static PW_A_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 127,
    x: &[0, 21, 42, 63, 84, 105, 127, 0],
    y: &[65, 69, 78, 90, 104, 115, 122],
    a: &[0.0297619998, 0.012755, 0.00680299988, -0.092840001, 0.0144819999, 0.0172700007, 0.0],
    b: &[0.0892859995, 0.0765310004, 0.0340140015, 0.140459001, -0.112118997, -0.139743999, 0.0],
    c: &[0.0714289993, 0.339286, 0.530611992, 0.619047999, 0.621446013, 0.440656006, 0.212977007],
};

/// Curve used by PW_B.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static PW_B_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 127,
    x: &[0, 21, 42, 63, 84, 105, 127, 0],
    y: &[0, 4, 9, 14, 19, 24, 28],
    a: &[0.0238099992, 0.0, 0.119047999, 0.119047999, 0.0, 0.0274839997, 0.0],
    b: &[0.0, 0.0, -0.119047999, -0.238095, 0.0, -0.0837609991, 0.0],
    c: &[0.166666999, 0.238095, 0.238095, 0.357143015, 0.238095, 0.238095, 0.153025001],
};

/// Curve used by HPF.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static HPF_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 1023,
    x: &[0, 170, 341, 511, 682, 852, 1023, 0],
    y: &[50, 465, 591, 636, 667, 701, 756],
    a: &[-0.484099001, -0.0795380026, -0.0582079999, 0.0107930005, 0.00507699978, -0.0444630012, 0.0],
    b: &[-0.365568995, -0.29102999, 0.0361780003, -0.0139739998, 0.00602399977, 0.149922997, 0.0],
    c: &[3.29084396, 1.10740995, 0.286736012, 0.184467003, 0.188898996, 0.216177002, 0.382634997],
};

/// Curve used by FREQ.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static FREQ_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 1023,
    x: &[0, 170, 341, 511, 682, 852, 1023, 0],
    y: &[10, 192, 367, 533, 700, 887, 959],
    a: &[-0.000102999998, -0.0232209992, 0.000174000001, -7.50000036e-05, -0.227523997, 0.234320998, 0.0],
    b: &[-0.0234249998, -0.000342999992, -0.000311000011, 0.000174000001, 0.350890994, -0.809110999, 0.0],
    c: &[1.09411705, 1.04695594, 0.976607025, 0.976508975, 0.976633012, 0.995842993, 0.0805829987],
};

/// Curve used by RESO.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static RESO_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 1023,
    x: &[0, 170, 341, 511, 682, 852, 1023, 0],
    y: &[0, 150, 307, 522, 654, 788, 829],
    a: &[-0.0144969998, -0.00897900015, -0.793753028, 0.0298590008, -0.0243790001, 0.257640004, 0.0],
    b: &[0.0323319994, 0.0414160006, 1.116871, -0.0520009995, 0.0329679996, -0.790319026, 0.0],
    c: &[0.864517987, 0.885692, 0.941586971, 0.794071972, 0.779645979, 0.772445023, -0.035273999],
};

/// Curve used by FENV_DEPTH.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static FENV_DEPTH_CURVE: MdlCurve = MdlCurve {
    signed: true,
    lo: 0,
    hi: 1023,
    x: &[0, 170, 341, 511, 682, 852, 1023, 0],
    y: &[0, 378, 756, 1323, 1764, 2268, 2394],
    a: &[0.00633500004, 0.0318650007, -1.66894901, 0.290926009, -2.51585197, 0.0, 0.0],
    b: &[-0.012817, -0.0447200015, 2.77470589, -0.484082013, 2.80383992, -0.0, 0.0],
    c: &[2.23001194, 2.22338104, 2.22953606, 2.77210307, 2.676718, 0.736841977, 0.736841977],
};

/// Curve used by FILTER_MOD.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static FILTER_MOD_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 100,
    x: &[0, 16, 33, 50, 66, 83, 100, 0],
    y: &[0, 8, 20, 25, 30, 35, 40],
    a: &[-0.0325950012, -0.516668975, 0.0513480008, -0.0191680007, 0.0, 0.147058994, 0.0],
    b: &[0.132416993, 0.655324996, -0.0850989968, 0.0199529994, -0.0, -0.147058994, 0.0],
    c: &[0.400177985, 0.567227006, 0.327868998, 0.311713994, 0.294117987, 0.294117987, 0.441175997],
};

/// Curve used by KEY_FOLLOW.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static KEY_FOLLOW_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 200,
    x: &[0, 33, 66, 100, 133, 166, 200, 0],
    y: &[0, 16, 32, 48, 64, 80, 100],
    a: &[0.242423996, 0.0, 0.0285209995, 0.0, 0.0, -0.0509210005, 0.0],
    b: &[-0.484847993, 0.0, -0.0427809991, 0.0, 0.0, 0.154308006, 0.0],
    c: &[0.727272987, 0.484847993, 0.484847993, 0.484847993, 0.484847993, 0.484847993, 0.640700996],
};

/// Curve used by AMP_LEVEL.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static AMP_LEVEL_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 127,
    x: &[0, 21, 42, 63, 84, 105, 127, 0],
    y: &[26, 52, 78, 100, 112, 124, 127],
    a: &[0.619047999, 0.0, -0.285714, 0.0, 0.0, 0.212475002, 0.0],
    b: &[-1.23809505, 0.0, 0.0952380002, 0.0, 0.0, -0.647539973, 0.0],
    c: &[1.85714304, 1.23809505, 1.23809505, 0.571429014, 0.571429014, 0.571429014, -0.0862260014],
};

/// Curve used by ENV1_ATTACK.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static ENV1_ATTACK_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 1023,
    x: &[0, 170, 341, 511, 682, 852, 1023, 0],
    y: &[0, 2, 20, 139, 412, 598, 730],
    a: &[-0.0339419991, -0.0246590003, -0.0525849983, -0.78006798, -0.0632570013, -0.0354500003, 0.0],
    b: &[0.080554001, 0.105488002, 0.591153026, 1.19057596, -0.0695570037, -0.0906649977, 0.0],
    c: &[-0.0348469988, 0.0244340003, 0.161431998, 1.18598199, 1.22693205, 0.898046017, 0.61036402],
};

/// Curve used by ENV1_DECAY.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static ENV1_DECAY_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 1023,
    x: &[0, 170, 341, 511, 682, 852, 1023, 0],
    y: &[1, 50, 159, 572, 781, 915, 1023],
    a: &[-0.117275998, 0.109668002, -2.361763, -0.148778006, -0.0651680008, -0.0369179994, 0.0],
    b: &[0.291359991, 0.182715997, 3.75169492, -0.0865819976, 0.015319, -0.0047220001, 0.0],
    c: &[0.114151999, 0.345043004, 1.03947902, 1.457582, 0.838084996, 0.673218012, 0.553022027],
};

/// Curve used by ENV1_SUSTAIN_F.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static ENV1_SUSTAIN_F_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 1023,
    x: &[0, 170, 341, 511, 682, 852, 1023, 0],
    y: &[0, 200, 417, 609, 810, 950, 1023],
    a: &[-0.00930900034, -0.148937002, 0.0594419986, -0.0695459992, 0.129141003, 0.011554, 0.0],
    b: &[0.0554409996, 0.20465, -0.105811, 0.102499999, -0.444458991, -0.222005993, 0.0],
    c: &[1.13033903, 1.21329296, 1.17578101, 1.14248395, 1.13884795, 0.63735199, 0.228002995],
};

/// Curve used by ENV1_RELEASE.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static ENV1_RELEASE_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 1023,
    x: &[0, 170, 341, 511, 682, 852, 1023, 0],
    y: &[0, 200, 410, 582, 846, 950, 1023],
    a: &[-0.0158089995, -0.0608070008, 0.297405988, -1.12363195, 0.0861430019, -0.0620319992, 0.0],
    b: &[0.0415330008, 0.102490999, -0.494589001, 1.55550504, -0.326478004, 0.0313609987, 0.0],
    c: &[1.15074694, 1.18638504, 1.20894802, 1.11198699, 0.852101028, 0.457572013, 0.334197015],
};

/// Curve used by ATTACK.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static ATTACK_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 1023,
    x: &[0, 170, 341, 511, 682, 852, 1023, 0],
    y: &[0, 10, 27, 192, 461, 669, 730],
    a: &[-0.0184339993, 0.0161979999, -0.386209011, -0.315880001, -0.0423640013, 0.183541998, 0.0],
    b: &[0.0386710018, 0.0225920007, 1.20239496, 0.48841399, -0.163859993, -0.801757991, 0.0],
    c: &[0.0385870002, 0.0606260002, 0.154402003, 1.40056503, 1.42975402, 0.974941015, -0.0779479966],
};

/// Curve used by ENV2_DECAY.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static ENV2_DECAY_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 1023,
    x: &[0, 170, 341, 511, 682, 852, 1023, 0],
    y: &[1, 15, 137, 634, 786, 920, 1023],
    a: &[0.0, -0.107873999, -3.63286901, -0.00359999994, -0.056747999, 0.0275010001, 0.0],
    b: &[0.0, 0.738972008, 5.31972504, -0.0850270018, 0.0483210012, -0.148223996, 0.0],
    c: &[0.0823530033, 0.0823530033, 1.236673, 0.977515996, 0.796661973, 0.723061025, 0.509118974],
};

/// Curve used by ENV2_SUSTAIN.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static ENV2_SUSTAIN_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 1023,
    x: &[0, 170, 341, 511, 682, 852, 1023, 0],
    y: &[0, 72, 292, 517, 722, 914, 1023],
    a: &[0.397215009, -0.00324900006, -0.0480090007, 0.0673220009, -0.00529799983, 0.184590995, 0.0],
    b: &[0.0330299996, 0.0388099998, 0.0526780002, -0.148680001, -0.0500839986, -0.615897, 0.0],
    c: &[-0.00671500014, 1.25098896, 1.31886101, 1.28018904, 1.184793, 1.06873298, 0.390713006],
};

/// Curve used by RELEASE.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static RELEASE_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 1023,
    x: &[0, 170, 341, 511, 682, 852, 1023, 0],
    y: &[0, 50, 101, 497, 715, 900, 1023],
    a: &[-0.00205000001, 0.00380000006, -2.98906708, -0.0495139994, -0.0753230006, 0.0602370016, 0.0],
    b: &[0.00410699984, 0.000319000013, 5.01231289, -0.0392230004, 0.0269559994, -0.30548501, 0.0],
    c: &[0.292059988, 0.294126004, 0.30616501, 1.36359096, 1.13660204, 0.964545012, 0.534289002],
};

/// Curve used by BENDER_FILTER.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static BENDER_FILTER_CURVE: MdlCurve = MdlCurve {
    signed: true,
    lo: 0,
    hi: 63,
    x: &[0, 10, 21, 31, 42, 52, 63, 0],
    y: &[0, 3, 6, 9, 12, 15, 18],
    a: &[-0.000316999998, 0.0276050009, -0.0272729993, 0.0272729993, -0.0269559994, -0.000333000004, 0.0],
    b: &[-0.0126700001, -0.0415740013, 0.0409089997, -0.0409089997, 0.0405919999, -0.0136209996, 0.0],
    c: &[0.312987, 0.286695987, 0.286363989, 0.286363989, 0.286363989, 0.286680996, 0.258441001],
};

/// Curve used by MOD_LFO.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static MOD_LFO_CURVE: MdlCurve = MdlCurve {
    signed: true,
    lo: 0,
    hi: 63,
    x: &[0, 10, 21, 31, 42, 52, 63, 0],
    y: &[0, 8, 21, 33, 42, 46, 47],
    a: &[0.181817994, -0.00909100007, -0.0250000004, 0.134809002, 0.012879, -0.0270150006, 0.0],
    b: &[0.0, 0.0272729993, 0.0340909995, -0.500717998, -0.199959993, -0.107873999, 0.0],
    c: &[0.618182003, 1.16363597, 1.19090903, 1.18409097, 0.587081015, 0.225797996, -0.070995003],
};

/// Curve used by PORTA_TIME.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static PORTA_TIME_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 1023,
    x: &[0, 102, 204, 306, 409, 511, 613, 716, 818, 920, 1023, 0],
    y: &[0, 1, 11, 87, 205, 338, 440, 513, 565, 586, 601],
    a: &[-0.0335289985, 0.0391639993, -0.208439007, 0.0112749999, -0.175303996, 0.0208720006, 0.0178740006, -0.131687999, 0.0204080008, -0.0202619992, 0.0],
    b: &[0.0776470006, 0.0384830013, 0.738686979, 0.0674490035, 0.243595004, -0.217781007, -0.133100003, 0.0301050004, -0.0910589993, 0.0102530001, 0.0],
    c: &[-0.0343139991, 0.0203920007, 0.214849994, 1.06690705, 1.23563099, 1.19690895, 0.823964, 0.611387014, 0.276533008, 0.155640006, 0.115360998],
};

/// Curve used by ENV1_SUSTAIN_P.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static ENV1_SUSTAIN_P_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 1023,
    x: &[0, 170, 341, 511, 682, 852, 1023, 0],
    y: &[0, 85, 170, 255, 340, 425, 511],
    a: &[-1.99999999e-06, 0.00292599993, -0.00292399991, 0.00292399991, -1.99999999e-06, 1.99999999e-06, 0.0],
    b: &[-0.00145600003, -0.0043899999, 0.00438599987, -0.00438599987, 0.00146399997, 0.00146199996, 0.0],
    c: &[0.501457989, 0.498540014, 0.498537987, 0.498537987, 0.498537987, 0.501460016, 0.504390001],
};

/// Curve used by OSC1_LEVEL, OSC2_LEVEL.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
pub static OSC1_LEVEL_CURVE: MdlCurve = MdlCurve {
    signed: false,
    lo: 0,
    hi: 255,
    x: &[0, 25, 51, 76, 102, 127, 153, 178, 204, 229, 255, 0],
    y: &[0, 44, 84, 119, 151, 178, 202, 221, 237, 248, 255],
    a: &[-0.0266929995, 0.00774999987, -0.0210510008, 0.00998700038, -0.00284500001, -0.00332799996, 0.00853199977, -0.00548300007, 0.00926900003, -0.00198299997, 0.0],
    b: &[-0.0819040015, -0.093998, -0.0389130004, -0.0982030034, -0.0696950033, -0.0782100037, -0.0867419988, -0.0694539994, -0.104234003, -0.0830919966, 0.0],
    c: &[1.86859703, 1.62470901, 1.45996404, 1.31898499, 1.15253997, 1.00461495, 0.838209987, 0.690321982, 0.534964025, 0.354304999, 0.182173997],
};

/// The ZEN-Core tone fields the JP-8 setters write, by the firmware's (block, parameter) pair.
/// `offset` is within the block instance; 0xFFFF where the layout does not name it.
#[rustfmt::skip]
pub static TONE_TARGETS: &[MdlTarget] = &[
    MdlTarget { block: 22, param: 0x705, offset: 20, size: 1 }, // PCMT_CMN LEVEL
    MdlTarget { block: 22, param: 0x70c, offset: 29, size: 1 }, // PCMT_CMN ANALOG_FEEL
    MdlTarget { block: 22, param: 0x70d, offset: 30, size: 1 }, // PCMT_CMN MONO_POLY
    MdlTarget { block: 22, param: 0x710, offset: 33, size: 1 }, // PCMT_CMN PORTA_SW
    MdlTarget { block: 22, param: 0x711, offset: 34, size: 1 }, // PCMT_CMN PORTA_MODE
    MdlTarget { block: 22, param: 0x714, offset: 38, size: 2 }, // PCMT_CMN PORTA_TIME
    MdlTarget { block: 22, param: 0x715, offset: 40, size: 1 }, // PCMT_CMN BEND_RANGE_UP
    MdlTarget { block: 22, param: 0x716, offset: 41, size: 1 }, // PCMT_CMN BEND_RANGE_DW
    MdlTarget { block: 22, param: 0x71d, offset: 48, size: 1 }, // PCMT_CMN PORTA_CRV
    MdlTarget { block: 25, param: 0x746, offset: 2, size: 1 }, // PCMT_PTL PIT_CRS
    MdlTarget { block: 25, param: 0x747, offset: 3, size: 1 }, // PCMT_PTL PIT_FINE
    MdlTarget { block: 25, param: 0x751, offset: 14, size: 2 }, // PCMT_PTL DLY_TIME
    MdlTarget { block: 25, param: 0x761, offset: 34, size: 2 }, // PCMT_PTL PIT_KF
    MdlTarget { block: 25, param: 0x764, offset: 40, size: 2 }, // PCMT_PTL CUTOFF
    MdlTarget { block: 25, param: 0x765, offset: 42, size: 2 }, // PCMT_PTL CUTOFF_KF
    MdlTarget { block: 25, param: 0x768, offset: 46, size: 2 }, // PCMT_PTL RESO
    MdlTarget { block: 25, param: 0x770, offset: 55, size: 1 }, // PCMT_PTL FENV_TKF
    MdlTarget { block: 25, param: 0x771, offset: 56, size: 1 }, // PCMT_PTL AENV_TKF
    MdlTarget { block: 25, param: 0x77b, offset: 66, size: 1 }, // PCMT_PTL ?54
    MdlTarget { block: 25, param: 0x780, offset: 74, size: 1 }, // PCMT_PTL ?59
    MdlTarget { block: 25, param: 0x789, offset: 65535, size: 0 }, // PCMT_PTL ?68
    MdlTarget { block: 25, param: 0x792, offset: 65535, size: 0 }, // PCMT_PTL ?77
    MdlTarget { block: 25, param: 0x794, offset: 65535, size: 0 }, // PCMT_PTL ?79
    MdlTarget { block: 25, param: 0x796, offset: 65535, size: 0 }, // PCMT_PTL ?81
    MdlTarget { block: 26, param: 0x6ce, offset: 0, size: 1 }, // PTL_PENV DEPTH
    MdlTarget { block: 26, param: 0x6d2, offset: 4, size: 2 }, // PTL_PENV T1
    MdlTarget { block: 26, param: 0x6d4, offset: 8, size: 2 }, // PTL_PENV T3
    MdlTarget { block: 26, param: 0x6d5, offset: 10, size: 2 }, // PTL_PENV T4
    MdlTarget { block: 26, param: 0x6d9, offset: 18, size: 2 }, // PTL_PENV L3
    MdlTarget { block: 27, param: 0x6dc, offset: 0, size: 1 }, // PTL_FENV DEPTH
    MdlTarget { block: 27, param: 0x6e1, offset: 6, size: 2 }, // PTL_FENV T1
    MdlTarget { block: 27, param: 0x6e3, offset: 10, size: 2 }, // PTL_FENV T3
    MdlTarget { block: 27, param: 0x6e4, offset: 12, size: 2 }, // PTL_FENV T4
    MdlTarget { block: 27, param: 0x6e8, offset: 20, size: 2 }, // PTL_FENV L3
    MdlTarget { block: 28, param: 0x6ec, offset: 2, size: 2 }, // PTL_AENV T1
    MdlTarget { block: 28, param: 0x6ee, offset: 6, size: 2 }, // PTL_AENV T3
    MdlTarget { block: 28, param: 0x6ef, offset: 8, size: 2 }, // PTL_AENV T4
    MdlTarget { block: 28, param: 0x6f2, offset: 14, size: 2 }, // PTL_AENV L3
    MdlTarget { block: 29, param: 0x66c, offset: 0, size: 1 }, // PTL_LFO LFO_1_FORM
    MdlTarget { block: 29, param: 0x670, offset: 4, size: 2 }, // PTL_LFO LFO_1_RATE
    MdlTarget { block: 29, param: 0x673, offset: 8, size: 2 }, // PTL_LFO LFO_1_DELAY
    MdlTarget { block: 29, param: 0x676, offset: 12, size: 2 }, // PTL_LFO LFO_1_FADE
    MdlTarget { block: 29, param: 0x678, offset: 15, size: 1 }, // PTL_LFO LFO_1_PIT_DEPTH
    MdlTarget { block: 29, param: 0x679, offset: 16, size: 1 }, // PTL_LFO LFO_1_TVF_DEPTH
    MdlTarget { block: 29, param: 0x67a, offset: 17, size: 1 }, // PTL_LFO LFO_1_TVA_DEPTH
    MdlTarget { block: 29, param: 0x69d, offset: 52, size: 1 }, // PTL_LFO LFO_2_FORM
    MdlTarget { block: 29, param: 0x6a1, offset: 56, size: 2 }, // PTL_LFO LFO_2_RATE
    MdlTarget { block: 31, param: 0x841, offset: 0, size: 1 }, // PCMS_CMN UNISON_SW
    MdlTarget { block: 31, param: 0x843, offset: 2, size: 1 }, // PCMS_CMN UNISON_DETN
    MdlTarget { block: 31, param: 0x844, offset: 4, size: 1 }, // PCMS_CMN BEND_RANGE_FINE_UP
    MdlTarget { block: 31, param: 0x845, offset: 5, size: 1 }, // PCMS_CMN BEND_RANGE_FINE_DW
    MdlTarget { block: 31, param: 0x846, offset: 6, size: 1 }, // PCMS_CMN RND_PIT_VAL
    MdlTarget { block: 31, param: 0x848, offset: 8, size: 1 }, // PCMS_CMN CONDITION
    MdlTarget { block: 32, param: 0x849, offset: 0, size: 1 }, // PCMS_PMT STRUCT12
    MdlTarget { block: 32, param: 0x84b, offset: 4, size: 1 }, // PCMS_PMT RING12_LEVEL
    MdlTarget { block: 32, param: 0x851, offset: 12, size: 2 }, // PCMS_PMT XMOD12_DEPTH
    MdlTarget { block: 32, param: 0x853, offset: 16, size: 1 }, // PCMS_PMT XMOD_OSC1_LEVEL
    MdlTarget { block: 32, param: 0x854, offset: 17, size: 1 }, // PCMS_PMT XMOD_OSC2_LEVEL
    MdlTarget { block: 33, param: 0x85a, offset: 0, size: 1 }, // PCMS_PTL OSC_TYPE
    MdlTarget { block: 33, param: 0x85b, offset: 1, size: 1 }, // PCMS_PTL VA_FORM
    MdlTarget { block: 33, param: 0x85c, offset: 2, size: 2 }, // PCMS_PTL SYNC_WAV_NUM
    MdlTarget { block: 33, param: 0x85d, offset: 4, size: 1 }, // PCMS_PTL PW
    MdlTarget { block: 33, param: 0x85e, offset: 5, size: 1 }, // PCMS_PTL PWM_DEPTH
    MdlTarget { block: 33, param: 0x861, offset: 8, size: 2 }, // PCMS_PTL HPF_CUTOFF
    MdlTarget { block: 33, param: 0x863, offset: 11, size: 1 }, // PCMS_PTL FILTER_SLOPE
    MdlTarget { block: 33, param: 0x866, offset: 14, size: 1 }, // PCMS_PTL VCF_TYPE
    MdlTarget { block: 33, param: 0x867, offset: 15, size: 1 }, // PCMS_PTL OSC_ATT
    MdlTarget { block: 33, param: 0x86a, offset: 18, size: 1 }, // PCMS_PTL FENV_FINE_DEPTH
    MdlTarget { block: 33, param: 0x86e, offset: 22, size: 1 }, // PCMS_PTL PITCH_DOWN_DEPTH
];

/// Start of each instance of a block in the tone (tone-relative, add 0x10 for the record).
#[rustfmt::skip]
pub static TONE_BLOCKS: &[(u8, &[u16])] = &[
    (22, &[0x0]), // PCMT_CMN
    (25, &[0xc8, 0x144, 0x1c0, 0x23c]), // PCMT_PTL
    (26, &[0x2b8, 0x2d0, 0x2e8, 0x300]), // PTL_PENV
    (27, &[0x318, 0x330, 0x348, 0x360]), // PTL_FENV
    (28, &[0x378, 0x388, 0x398, 0x3a8]), // PTL_AENV
    (29, &[0x3b8, 0x420, 0x488, 0x4f0]), // PTL_LFO
    (31, &[0x598]), // PCMS_CMN
    (32, &[0x5ac]), // PCMS_PMT
    (33, &[0x5d0, 0x5f4, 0x618, 0x63c]), // PCMS_PTL
];
