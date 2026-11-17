//! Animações: a mesma curva do AbyssSub (cubic-bezier(.2,.8,.2,1)) e um
//! "animador" simples que lembra o valor de cada coisa que se mexe na tela.

use std::collections::HashMap;

fn bez(a: f32, b: f32, s: f32) -> f32 {
    let u = 1.0 - s;
    3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s
}

/// cubic-bezier(x1, y1, x2, y2) do CSS.
pub fn bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let (mut lo, mut hi, mut s) = (0.0f32, 1.0f32, t);
    for _ in 0..24 {
        let x = bez(x1, x2, s);
        if (x - t).abs() < 1e-4 {
            break;
        }
        if x < t {
            lo = s;
        } else {
            hi = s;
        }
        s = (lo + hi) / 2.0;
    }
    bez(y1, y2, s)
}

/// A curva "--ease" do AbyssSub: rápida no começo, bem suave no fim.
pub fn suave(t: f32) -> f32 {
    bezier(0.2, 0.8, 0.2, 1.0, t.clamp(0.0, 1.0))
}

/// ease-in-out (usado no brilho do topo).
pub fn vai_e_volta(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

struct Trans {
    de: f32,
    para: f32,
    inicio: f64,
    dur: f32,
}

impl Trans {
    fn prog(&self, agora: f64) -> f32 {
        if self.dur <= 0.0 {
            1.0
        } else {
            (((agora - self.inicio) as f32) / self.dur).clamp(0.0, 1.0)
        }
    }
    fn valor(&self, agora: f64) -> f32 {
        self.de + (self.para - self.de) * suave(self.prog(agora))
    }
}

#[derive(Default)]
pub struct Animador {
    mapa: HashMap<String, Trans>,
    pub agora: f64,
    /// Alguma animação ainda está rodando neste quadro (precisa redesenhar).
    pub ativo: bool,
    /// Animações desligadas nas configurações: tudo pula direto para o fim.
    pub desligado: bool,
}

impl Animador {
    pub fn quadro(&mut self, agora: f64) {
        self.agora = agora;
        self.ativo = false;
    }

    /// Vai suavemente até `alvo`. Na primeira vez já começa no alvo.
    pub fn ir(&mut self, chave: &str, alvo: f32, dur: f32) -> f32 {
        self.ir_de(chave, alvo, alvo, dur)
    }

    /// Igual a `ir`, mas na primeira vez começa em `inicial` (ex.: contar do zero).
    pub fn ir_de(&mut self, chave: &str, inicial: f32, alvo: f32, dur: f32) -> f32 {
        let agora = self.agora;
        let t = self.mapa.entry(chave.to_string()).or_insert(Trans {
            de: inicial,
            para: alvo,
            inicio: agora,
            dur,
        });
        if (t.para - alvo).abs() > 1e-6 {
            t.de = t.valor(agora);
            t.para = alvo;
            t.inicio = agora;
            t.dur = dur;
        }
        if self.desligado {
            t.de = alvo;
            t.inicio = agora - t.dur as f64;
            return alvo;
        }
        let v = t.valor(agora);
        let rodando = t.prog(agora) < 1.0;
        if rodando {
            self.ativo = true;
        }
        v
    }

    /// Progresso 0..1 (já com a curva) de algo que começou em `inicio` + `atraso`.
    pub fn desde(&mut self, inicio: f64, atraso: f32, dur: f32) -> f32 {
        if self.desligado {
            return 1.0;
        }
        let p = (((self.agora - inicio) as f32) - atraso) / dur;
        if p < 1.0 {
            self.ativo = true;
        }
        suave(p.clamp(0.0, 1.0))
    }

    /// Esquece os valores cuja chave começa com `prefixo` (voltam a animar do início).
    pub fn esquecer_prefixo(&mut self, prefixo: &str) {
        self.mapa.retain(|k, _| !k.starts_with(prefixo));
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn curva_nas_pontas() {
        assert_eq!(suave(0.0), 0.0);
        assert_eq!(suave(1.0), 1.0);
        assert!(suave(0.5) > 0.8, "a curva do AbyssSub anda rápido no começo");
        let mut ant = 0.0;
        for i in 0..=100 {
            let v = suave(i as f32 / 100.0);
            assert!(v >= ant - 1e-4);
            ant = v;
        }
    }

    #[test]
    fn animador_vai_ate_o_alvo() {
        let mut a = Animador::default();
        a.quadro(0.0);
        assert_eq!(a.ir_de("x", 0.0, 10.0, 1.0), 0.0);
        a.quadro(0.5);
        let meio = a.ir_de("x", 0.0, 10.0, 1.0);
        assert!(meio > 5.0 && meio < 10.0);
        assert!(a.ativo);
        a.quadro(2.0);
        assert_eq!(a.ir_de("x", 0.0, 10.0, 1.0), 10.0);
        assert!(!a.ativo);
        // mudar o alvo continua de onde está
        assert_eq!(a.ir("x", 20.0, 1.0), 10.0);
    }
}
