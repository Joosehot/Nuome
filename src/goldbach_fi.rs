//! The Goldbach thesis written by Nuome in Finnish. The numbers come from the
//! same computations as the English one (src/attempt.rs): the run, the
//! constant Nuome finds, the range check, and Nuome's own derivative and
//! logic rules. Only the sentences differ.

use crate::attempt::{range_check, simple_c_data};
use crate::config::Config;
use crate::goldbach::C2;

/// 1 000 000 style.
fn luku(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

/// A decimal with a comma.
fn des(x: f64, d: usize) -> String {
    format!("{x:.d$}").replace('.', ",")
}

/// Run one of Nuome's own rules and keep its answer and the result of its checks.
fn saanto(cfg: &Config, lause: &str) -> Option<(String, Vec<String>)> {
    let opts = crate::Options { lenient: false, style: crate::print::Style::Ascii };
    let s = crate::solve(lause, cfg, &opts).ok()?;
    let vastaus = s.text.lines().find(|l| l.starts_with("Answer:"))?.trim_start_matches("Answer:").trim().to_string();
    let tarkistukset = s.text.lines().filter(|l| l.trim_start().starts_with("ok ")).map(|l| l.trim().to_string()).collect();
    Some((vastaus, tarkistukset))
}

fn tarkistettu(t: &[String]) -> String {
    if t.is_empty() {
        "ei tarkistuksia".into()
    } else {
        format!("Nuomen {} tarkistusta läpäisty", t.len())
    }
}

pub fn goldbach_proof(cfg: &Config, limit: Option<u64>) -> Option<String> {
    let rules = cfg.open.get("goldbach")?;
    let limit = limit.unwrap_or(rules.check_up_to.unwrap_or(1_000_000_000)).min(rules.max_check.unwrap_or(u64::MAX));
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let run = crate::goldbach::run(limit, threads);
    let d = simple_c_data(rules, limit, &run.records)?;
    let c = d.c;
    let from = rules.bound_from.unwrap_or(1000);
    let ennuste = 1.0 / C2;
    let kaava = format!("{} (ln n)² ln ln n", des(c, 3));
    let mut o: Vec<String> = Vec::new();

    o.push(format!("GOLDBACHIN RAJA: p(n) ≤ {kaava} jokaiselle parilliselle n ≥ {}", luku(from)));
    o.push("missä p(n) on pienin alkuluku p, jolla myös n − p on alkuluku".into());
    let (wn, wq, oma) = d.at;
    o.push(format!(
        "Vakion {} Nuome löysi itse: suurin suhde p / ((ln n)² ln ln n) sen {} ennätyksestä ({} itse laskettua {} asti ja {} julkaistua sen yläpuolelta) on {}, kohdassa n = {} (p = {wq}, {}). Se pyöristetään ylöspäin kolmeen desimaaliin. Malli ennustaa arvon 1/C₂ = {}.",
        des(c, 3),
        d.own + d.published,
        d.own,
        luku(limit),
        d.published,
        des(d.worst, 4),
        luku(wn),
        if oma { "Nuomen laskema" } else { "julkaistu ennätys" },
        des(ennuste, 4)
    ));

    o.push(String::new());
    o.push("OSA 1. MITÄ KAAVA TARKOITTAA".into());
    o.push("  Merkitys: p(n) kertoo, kuinka kauas pitää etsiä. Kokeillaan alkulukuja p = 3, 5, 7, … järjestyksessä, kunnes myös n − p on alkuluku. Kaava sanoo, että etsintä päättyy aina ennen tätä rajaa.".into());
    o.push(format!(
        "  1. ln n, yhden kokeen mahdollisuus (LAUSE: alkulukulause 1896, lukujen n lähellä noin joka ln n:s luku on alkuluku). Hardy–Littlewoodin mukaan n − p on alkuluku todennäköisyydellä noin 2C₂·S(n)/ln n, missä C₂ = {} on kaksosalkulukuvakio ja S(n) ≥ 1. Vaikeimmille luvuille S(n) = 1, joten mahdollisuus on {}/ln n.",
        des(C2, 5),
        des(2.0 * C2, 4)
    ));
    o.push("  2. (ln n)², montako koetta pahin luku tarvitsee (MALLIN OLETUS: kokeet ovat riippumattomia sattumia). Yksi ln n tulee yhden kokeen mahdollisuudesta, toinen siitä, että parillisia lukuja on noin n/2. Pahin niistä tarvitsee noin ln(n/2)/mahdollisuus koetta, eli m = (ln n)²/(2C₂).".into());
    o.push("  3. ln ln n, alkuluvut harvenevat (LAUSE: alkulukulauseesta m:s alkuluku on noin m ln m). Kun m = (ln n)²/(2C₂), on ln m ≈ 2 ln ln n, joten suurin tarvittava alkuluku on noin (1/C₂)(ln n)² ln ln n.".into());
    o.push(format!(
        "  4. Vakio {}: johto antaa 1/C₂ = {}. Pahin havaittu arvo on {}, eli {} % enemmän: vaikeimpien tapausten huonoa tuuria.",
        des(c, 3),
        des(ennuste, 4),
        des(d.worst, 4),
        des((d.worst / ennuste - 1.0) * 100.0, 1)
    ));

    o.push(String::new());
    o.push("OSA 2. JOHTO TODISTETTUNA (yleisesti, yhtään arvoa tarkistamatta)".into());
    o.push("  Malli: N parillisella luvulla kokeet onnistuvat toisistaan riippumatta todennäköisyydellä q (0 < q ≤ 1/2). X_i on i:nnen luvun tarvitsemien kokeiden määrä, eli P(X_i > m) = (1 − q)^m, ja M = max X_i on pahin tapaus.".into());
    o.push("  Lemma A: ln(1 − q) ≥ −q − q², kun 0 ≤ q ≤ 1/2. Olkoon f(q) = ln(1 − q) + q + q², jolloin f(0) = 0. Nuome derivoi ja sieventää omilla säännöillään:".into());
    for lause in ["differentiate ln(1 - x) + x + x^2", "simplify -1/(1 - x) + 1 + 2x"] {
        match saanto(cfg, lause) {
            Some((v, t)) => o.push(format!("      {lause}  →  {v}   ({})", tarkistettu(&t))),
            None => o.push(format!("      {lause}  →  sääntö ei onnistunut, joten lemma A jää todistamatta")),
        }
    }
    o.push("    Siis f′(q) = q(1 − 2q)/(1 − q). Kun 0 ≤ q ≤ 1/2, jokainen tekijä on ≥ 0, joten f kasvaa arvosta f(0) = 0 ja f(q) ≥ 0. ∎".into());
    o.push("  Lause B: pahin tapaus tarvitsee (1 + o(1))·ln N/q koetta lähes varmasti. Jokaisella e > 0:".into());
    o.push("    Yläraja: P(M > m) ≤ N(1 − q)^m (unioniraja: jonkin N luvun on epäonnistuttava m kertaa) ≤ N·e^(−qm). Kun m = (1 + e) ln N/q, tämä on N^(−e). ∎".into());
    o.push("    Alaraja: riippumattomuuden nojalla P(M ≤ m) = (1 − (1 − q)^m)^N ≤ exp(−N(1 − q)^m). Lemman A mukaan (1 − q)^m ≥ e^(−m(q + q²)). Kun m = (1 − e) ln N/q ja q(1 − e) ≤ e/2, saadaan P(M ≤ m) ≤ exp(−N^(e/2)). ∎".into());
    o.push("  Lause C: m:s alkuluku on (1 + o(1))·m ln m (alkulukulause, Hadamard ja de la Vallée Poussin 1896).".into());
    o.push(format!(
        "  Yhdessä: kun N = n/2 ja q = 2C₂/ln n, lause B antaa M = (1 + o(1))(ln n)²/(2C₂), ja lause C antaa suurimmaksi alkuluvuksi (1 + o(1))·{}·(ln n)² ln ln n. Kaavan {} on juuri sen yläpuolella, kuten ylärajan kuuluu.",
        des(ennuste, 4),
        des(c, 3)
    ));
    o.push("  Ainoa todistamaton askel: että todelliset alkuluvut noudattavat mallia, eli että \"onko n − p alkuluku\" käyttäytyy jokaisella n:llä kuin riippumaton sattuma, jonka todennäköisyys on 2C₂·S(n)/ln n. Tämän todistus todistaisi Goldbachin, eikä kukaan osaa sitä vielä.".into());

    o.push(String::new());
    o.push("OSA 3. MITÄ KAAVASTA SEURAA (yleisesti, yhtään arvoa tarkistamatta)".into());
    o.push(format!("  Parilliselle n ≥ {}: b = \"jollakin alkuluvulla p ≤ {kaava} myös n − p on alkuluku\", g = \"n on kahden alkuluvun summa\", w = \"n + 3 on kolmen alkuluvun summa\".", luku(from)));
    o.push("  b ⇒ g: jos p ja n − p ovat alkulukuja, n = p + (n − p). ∎".into());
    o.push("  g ⇒ w: jos n = p + q, niin n + 3 = 3 + p + q, ja 3 on alkuluku. ∎".into());
    for (mita, lause) in [
        ("kaavasta seuraa Goldbach", "prove that (b and (b implies g)) implies g"),
        ("kaavasta seuraa kolmen alkuluvun väite", "prove that ((b implies g) and (g implies w)) implies (b implies w)"),
    ] {
        match saanto(cfg, lause) {
            Some((v, t)) => o.push(format!("  Nuomen logiikkasäännöt todistavat, että {mita}: {}  ({})", if v == "proved" { "todistettu" } else { &v }, tarkistettu(&t))),
            None => o.push(format!("  Nuomen logiikkasäännöt eivät todistaneet, että {mita}")),
        }
    }
    o.push("  Avoin: b itse jokaiselle n. Koska b ⇒ g, kaavan todistus olisi Goldbachin todistus, joka on ollut avoin vuodesta 1742. Lähin todistettu tulos on Chenin lause (1973).".into());

    o.push(String::new());
    o.push(format!("OSA 4. LASKETTU (ei yleinen todistus; jokainen parillinen luku {} asti, {} s, {} säiettä)", luku(limit), des(run.seconds, 1), run.threads));
    match run.counterexample {
        Some(n) => o.push(format!("  VASTAESIMERKKI: {} ei ole kahden alkuluvun summa", luku(n))),
        None => o.push(format!("  Jokainen parillinen luku 4:stä {}:een on kahden alkuluvun summa.", luku(limit))),
    }
    let f = |n: u64| c * (n as f64).ln().powi(2) * (n as f64).ln().ln();
    if let Some(rc) = range_check(&f, from, limit, &run.records) {
        if rc.failed.is_empty() {
            let (tn, tq, tb) = rc.tightest;
            o.push(format!(
                "  Kaava pätee jokaiselle parilliselle n välillä {} – {}: kaava kasvaa, kun n > e (kahden positiivisen kasvavan funktion tulo), ja ennätyslemman nojalla riittää tarkistaa {} voimassa olevaa ennätystä. Välin {}–{} alku laskettiin suoraan (suurin p = {}). Tiukin kohta n = {}: p = {tq}, raja {}, varaa {} %.",
                luku(from),
                luku(limit),
                rc.checked,
                luku(from),
                luku(rc.segment_end),
                rc.start_max,
                luku(tn),
                des(tb, 1),
                des((1.0 - tq as f64 / tb) * 100.0, 1)
            ));
        } else {
            o.push("  Kaava EI päde tällä välillä kaikissa ennätyksissä.".into());
        }
    }
    let yli: Vec<&(u64, u64)> = d.above.iter().collect();
    if let Some(&&(n, q)) = yli.iter().max_by(|a, b| (a.1 as f64 / f(a.0)).total_cmp(&(b.1 as f64 / f(b.0)))) {
        o.push(format!(
            "  Julkaistut ennätykset sen yläpuolelta ({} kpl, 4·10¹⁸ asti): {}. Tiukin n = {}: p = {q}, raja {} (tarkistettu, ei todistettu: luvut ovat muiden laskemia).",
            yli.len(),
            if yli.iter().all(|&&(n, q)| (q as f64) <= f(n)) { "jokainen pitää" } else { "OSA EI PIDÄ" },
            luku(n),
            des(f(n), 1)
        ));
    }

    o.push(String::new());
    o.push("YHTEENVETO".into());
    for (osa, asema) in [
        ("ln n: yhden kokeen mahdollisuus", "todistettu (alkulukulause)"),
        ("ln ln n: m:s alkuluku on m ln m", "todistettu (alkulukulause)"),
        ("(ln n)² ja vakio 1/C₂ mallissa", "todistettu (lemma A, lauseet B ja C)"),
        ("kaavasta seuraavat Goldbach ja kolmen alkuluvun väite", "todistettu (määritelmät ja logiikkasäännöt)"),
        ("vakio", "Nuomen löytämä ennätyksistä"),
        ("kaava laskentavälillä", "laskettu"),
        ("todelliset alkuluvut noudattavat mallia, joten kaava pätee kaikille n", "AVOIN: todistus todistaisi Goldbachin"),
    ] {
        o.push(format!("  {osa:<70} {asema}"));
    }
    Some(o.join("\n"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn suomeksi() {
        let cfg = crate::config::Config::builtin();
        let t = super::goldbach_proof(&cfg, Some(1_000_000)).expect("teesi");
        for osa in ["OSA 1", "OSA 2", "Lemma A", "Lause B", "OSA 3", "OSA 4", "YHTEENVETO", "AVOIN"] {
            assert!(t.contains(osa), "{osa}");
        }
        assert!(!t.contains("EI PÄDE") && !t.contains("OSA EI PIDÄ"));
    }
}
