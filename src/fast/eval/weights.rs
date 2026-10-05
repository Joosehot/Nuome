// Eval tables and hyperopt weights, ported VERBATIM from static/fast.js.
// Do not "improve" any value here — behavior parity is the goal.
// PST (a1=0), white perspective; index [0] unused, [1]=P [2]=N [3]=B [4]=R [5]=Q [6]=K.

pub const PST_MG: [[i32; 64]; 7] = [
    [0; 64],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 5, 10, 10, -20, -20, 10, 10, 5, 5, -5, -10, 0, 0, -10, -5, 5, 0,
        0, 0, 20, 20, 0, 0, 0, 5, 5, 10, 25, 25, 10, 5, 5, 10, 10, 20, 30, 30, 20, 10, 10, 50, 50,
        50, 50, 50, 50, 50, 50, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        -50, -40, -30, -30, -30, -30, -40, -50, -40, -20, 0, 5, 5, 0, -20, -40, -30, 5, 10, 15,
        15, 10, 5, -30, -30, 0, 15, 20, 20, 15, 0, -30, -30, 5, 15, 20, 20, 15, 5, -30, -30, 0,
        10, 15, 15, 10, 0, -30, -40, -20, 0, 0, 0, 0, -20, -40, -50, -40, -30, -30, -30, -30, -40,
        -50,
    ],
    [
        -20, -10, -10, -10, -10, -10, -10, -20, -10, 5, 0, 0, 0, 0, 5, -10, -10, 10, 10, 10, 10,
        10, 10, -10, -10, 0, 10, 10, 10, 10, 0, -10, -10, 5, 5, 10, 10, 5, 5, -10, -10, 0, 5, 10,
        10, 5, 0, -10, -10, 0, 0, 0, 0, 0, 0, -10, -20, -10, -10, -10, -10, -10, -10, -20,
    ],
    [
        0, 0, 0, 5, 5, 0, 0, 0, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0,
        0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, 5, 10, 10, 10, 10, 10,
        10, 5, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        -20, -10, -10, -5, -5, -10, -10, -20, -10, 0, 5, 0, 0, 0, 0, -10, -10, 5, 5, 5, 5, 5, 0,
        -10, 0, 0, 5, 5, 5, 5, 0, -5, -5, 0, 5, 5, 5, 5, 0, -5, -10, 0, 5, 5, 5, 5, 0, -10, -10,
        0, 0, 0, 0, 0, 0, -10, -20, -10, -10, -5, -5, -10, -10, -20,
    ],
    [
        20, 30, 10, 0, 0, 10, 30, 20, 20, 20, 0, 0, 0, 0, 20, 20, -10, -20, -20, -20, -20, -20,
        -20, -10, -20, -30, -30, -40, -40, -30, -30, -20, -30, -40, -40, -50, -50, -40, -40, -30,
        -30, -40, -40, -50, -50, -40, -40, -30, -30, -40, -40, -50, -50, -40, -40, -30, -30, -40,
        -40, -50, -50, -40, -40, -30,
    ],
];

pub const PST_EG: [[i32; 64]; 7] = [
    [0; 64],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 5, 5, 5, 5, 5, 5, 5, 5, 10, 10, 10, 10, 10, 10, 10, 10, 20, 20,
        20, 20, 20, 20, 20, 20, 30, 30, 30, 30, 30, 30, 30, 30, 50, 50, 50, 50, 50, 50, 50, 50,
        80, 80, 80, 80, 80, 80, 80, 80, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    PST_MG[2],
    PST_MG[3],
    PST_MG[4],
    PST_MG[5],
    [
        -50, -30, -30, -30, -30, -30, -30, -50, -30, -30, 0, 0, 0, 0, -30, -30, -30, -10, 20, 30,
        30, 20, -10, -30, -30, -10, 30, 40, 40, 30, -10, -30, -30, -10, 30, 40, 40, 30, -10, -30,
        -30, -10, 20, 30, 30, 20, -10, -30, -30, -20, -10, 0, 0, -10, -20, -30, -50, -40, -30,
        -20, -20, -30, -40, -50,
    ],
];

pub const PHASE: [i32; 7] = [0, 0, 1, 1, 2, 4, 0];

// 120-square lists (fast.js L191)
pub const CENTER4: [usize; 4] = [54, 55, 64, 65];
pub const EXT_CENTER: [usize; 12] = [43, 44, 45, 46, 53, 56, 63, 66, 73, 74, 75, 76];

// Hyperopt-tuned weights (fast.js W_, L189). All f64.
#[derive(Clone, Copy)]
pub struct Weights {
    pub mob: f64,
    pub trapped: f64,
    pub threat: f64,
    pub pin: f64,
    pub king_danger: f64,
    pub shield: f64,
    pub open_file: f64,
    pub pawn_doubled: f64,
    pub pawn_iso: f64,
    pub pawn_backward: f64,
    pub passed: f64,
    pub lever: f64,
    pub kick_pawn: f64,
    pub outpost: f64,
    pub outpost_b: f64,
    pub center_pawn: f64,
    pub center_atk: f64,
    pub space: f64,
    pub rook_open: f64,
    pub rook_semi: f64,
    pub rook7: f64,
    pub bishop_pair: f64,
    pub bad_bishop: f64,
    pub undev: f64,
    pub castled: f64,
    pub castled_q: f64,
    pub no_rights: f64,
    pub storm: f64,
    pub heavy_file: f64,
    pub early_q: f64,
    pub rim: f64,
    pub kick: f64,
    pub weak_own: f64,
    pub weak_enemy: f64,
    pub pawn_break: f64,
    pub break_line: f64,
    pub king_attack: f64,
    pub eg_king: f64,
    pub pos_scale: f64,
    pub dev_lag: f64,
    /// StarCelestials card weights.
    pub cel_king: f64,      // king escape squares, middlegame (cp per level)
    pub cel_king_eg: f64,   // king activity, endgame (cp per safe square)
    pub cel_report: f64,    // good-move report, side to move (share of gain)
    pub cel_report2: f64,   // second-best report, side to move
    pub cel_threat: f64,    // good-move report, other side
    pub cel_trap_n: f64,    // trapped knight (share of value)
    pub cel_trap_b: f64,    // trapped bishop
    pub cel_trap_r: f64,    // trapped rook
    pub cel_trap_q: f64,    // trapped queen
    pub mob_n: f64,         // per-piece mobility multipliers
    pub mob_b: f64,
    pub mob_r: f64,
    pub mob_q: f64,
    /// Share of positional compensation kept when a pawn or more down (opening/middlegame).
    pub mat_discipline: f64,
    /// StarCelestials on/off (0 = old eval exactly).
    pub celestials: f64,
    /// Threats tempo rule on/off (measured: error set 5/10 -> 3/10 at 3 s, off).
    pub tempo_rule: f64,
    /// Quiet-escape trap: share of a minor piece's value charged (raw, inside
    /// pos_scale*trapped) when it has no safe quiet square away from home.
    pub quiet_trap: f64,
    /// Shield phase floor when the enemy queen is present (0 = old eval exactly).
    pub shield_floor: f64,
    /// Material balance: flat cp for being ahead / behind in pure material.
    pub mat_balance: f64,
}

pub const W_DEFAULT: Weights = Weights {
    mob: 2.7961,
    trapped: 0.7314,
    threat: 0.4737,
    pin: 59.781,
    king_danger: 36.5697,
    shield: 13.7036,
    open_file: 25.0098,
    pawn_doubled: 9.9838,
    pawn_iso: 10.4197,
    pawn_backward: 23.2837,
    passed: 0.9112,
    lever: 10.7769,
    kick_pawn: 12.0387,
    outpost: 47.3898,
    outpost_b: 28.9672,
    center_pawn: 0.0219,
    center_atk: -0.0493,
    space: 3.7294,
    rook_open: 29.3375,
    rook_semi: 19.3255,
    rook7: 3.093,
    bishop_pair: 58.1038,
    bad_bishop: 1.9884,
    undev: 0.1052,
    castled: 0.1302,
    castled_q: -39.501,
    no_rights: 0.0235,
    storm: 6.0454,
    heavy_file: 23.1625,
    early_q: 20.2953,
    rim: -0.0765,
    kick: 32.5009,
    weak_own: 3.676,
    weak_enemy: 16.7379,
    pawn_break: 7.5818,
    break_line: 0.9329,
    king_attack: 0.0781,
    eg_king: 6.5944,
    pos_scale: 0.4538,
    dev_lag: 13.156,
    cel_king: 1.0,
    cel_king_eg: 4.0,
    cel_report: 0.35,
    cel_report2: 0.10,
    cel_threat: 0.05,
    cel_trap_n: 0.45,
    cel_trap_b: 0.45,
    cel_trap_r: 0.35,
    cel_trap_q: 0.25,
    mob_n: 3.9544,
    mob_b: 2.2946,
    mob_r: 2.3747,
    mob_q: 1.2437,
    mat_discipline: 0.5363,
    celestials: 0.0,
    tempo_rule: 0.0,
    quiet_trap: 0.0,
    shield_floor: 0.0,
    mat_balance: 65.6279,
};

/// Runtime weights: defaults unless STELDENS_WEIGHTS holds "name=value,name=value".
pub static W: std::sync::LazyLock<Weights> = std::sync::LazyLock::new(|| {
    let mut w = W_DEFAULT;
    if let Ok(s) = std::env::var("STELDENS_WEIGHTS") {
        for kv in s.split(',') {
            let Some((k, v)) = kv.split_once('=') else { continue };
            let Ok(v) = v.trim().parse::<f64>() else { continue };
            let slot: Option<&mut f64> = match k.trim() {
                "mob" => Some(&mut w.mob), "trapped" => Some(&mut w.trapped), "threat" => Some(&mut w.threat),
                "pin" => Some(&mut w.pin), "king_danger" => Some(&mut w.king_danger), "shield" => Some(&mut w.shield),
                "open_file" => Some(&mut w.open_file), "pawn_doubled" => Some(&mut w.pawn_doubled), "pawn_iso" => Some(&mut w.pawn_iso),
                "pawn_backward" => Some(&mut w.pawn_backward), "passed" => Some(&mut w.passed), "lever" => Some(&mut w.lever),
                "kick_pawn" => Some(&mut w.kick_pawn), "outpost" => Some(&mut w.outpost), "outpost_b" => Some(&mut w.outpost_b),
                "center_pawn" => Some(&mut w.center_pawn), "center_atk" => Some(&mut w.center_atk), "space" => Some(&mut w.space),
                "rook_open" => Some(&mut w.rook_open), "rook_semi" => Some(&mut w.rook_semi), "rook7" => Some(&mut w.rook7),
                "bishop_pair" => Some(&mut w.bishop_pair), "bad_bishop" => Some(&mut w.bad_bishop), "undev" => Some(&mut w.undev),
                "castled" => Some(&mut w.castled), "castled_q" => Some(&mut w.castled_q), "no_rights" => Some(&mut w.no_rights),
                "storm" => Some(&mut w.storm), "heavy_file" => Some(&mut w.heavy_file), "early_q" => Some(&mut w.early_q),
                "rim" => Some(&mut w.rim), "kick" => Some(&mut w.kick), "weak_own" => Some(&mut w.weak_own),
                "weak_enemy" => Some(&mut w.weak_enemy), "pawn_break" => Some(&mut w.pawn_break), "break_line" => Some(&mut w.break_line),
                "king_attack" => Some(&mut w.king_attack), "eg_king" => Some(&mut w.eg_king), "pos_scale" => Some(&mut w.pos_scale), "dev_lag" => Some(&mut w.dev_lag),
                "cel_king" => Some(&mut w.cel_king), "cel_report" => Some(&mut w.cel_report),
                "cel_king_eg" => Some(&mut w.cel_king_eg), "cel_report2" => Some(&mut w.cel_report2), "cel_threat" => Some(&mut w.cel_threat),
                "cel_trap_n" => Some(&mut w.cel_trap_n), "cel_trap_b" => Some(&mut w.cel_trap_b), "cel_trap_r" => Some(&mut w.cel_trap_r), "cel_trap_q" => Some(&mut w.cel_trap_q),
                "mob_n" => Some(&mut w.mob_n), "mob_b" => Some(&mut w.mob_b), "mob_r" => Some(&mut w.mob_r), "mob_q" => Some(&mut w.mob_q), "mat_discipline" => Some(&mut w.mat_discipline), "celestials" => Some(&mut w.celestials), "tempo_rule" => Some(&mut w.tempo_rule), "quiet_trap" => Some(&mut w.quiet_trap), "shield_floor" => Some(&mut w.shield_floor), "mat_balance" => Some(&mut w.mat_balance),
                _ => None,
            };
            if let Some(slot) = slot { *slot = v; }
        }
    }
    w
});
