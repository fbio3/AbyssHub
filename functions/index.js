// Servidor do AbyssHub: lê as horas de jogo da PlayStation Network.
//
// Uma conta PSN "robô" (do dono do app) lê o perfil PÚBLICO das pessoas, do mesmo jeito que sites
// como o YourGamerProfile. Ninguém entrega senha: para provar que a conta é dela, a pessoa coloca
// um código no "Sobre" do perfil, e o robô confere.
//
// O código de acesso do robô (NPSSO) fica no Secret Manager do Google (PSN_NPSSO), nunca no
// app nem no GitHub. Os tokens que ele gera ficam em abysshub_privado/psn_robo, que o app não
// consegue ler (as regras do Firestore não liberam essa coleção).

import { initializeApp } from "firebase-admin/app";
import { getFirestore, FieldValue } from "firebase-admin/firestore";
import { onCall, HttpsError } from "firebase-functions/v2/https";
import { defineSecret } from "firebase-functions/params";
import { logger } from "firebase-functions";
import {
  exchangeNpssoForAccessCode,
  exchangeAccessCodeForAuthTokens,
  exchangeRefreshTokenForAuthTokens,
  getProfileFromUserName,
  getProfileFromAccountId,
  getUserPlayedGames,
} from "psn-api";

initializeApp();
const db = getFirestore();
const PSN_NPSSO = defineSecret("PSN_NPSSO");

const CONEXOES = "abysshub_psn"; // um documento por pessoa (uid do Firebase)
const ROBO = db.collection("abysshub_privado").doc("psn_robo");
const INTERVALO_JOGOS_MS = 10 * 60 * 1000; // no máximo uma leitura da PSN a cada 10 min por pessoa
const INTERVALO_CODIGO_MS = 20 * 1000;

const opcoes = {
  region: "southamerica-east1",
  secrets: [PSN_NPSSO],
  maxInstances: 2,
  timeoutSeconds: 60,
  memory: "256MiB",
};

// ---------------------------------------------------------------------------
// Acesso do robô
// ---------------------------------------------------------------------------

/** Devolve { accessToken } válido, renovando quando precisa. */
async function acessoRobo() {
  const agora = Date.now();
  const salvo = (await ROBO.get()).data() ?? {};
  if (salvo.accessToken && salvo.accessExp > agora + 60_000) {
    return { accessToken: salvo.accessToken };
  }
  let tokens = null;
  if (salvo.refreshToken && salvo.refreshExp > agora + 60_000) {
    try {
      tokens = await exchangeRefreshTokenForAuthTokens(salvo.refreshToken);
    } catch (e) {
      logger.warn("Não deu para renovar o acesso da PSN, tentando com o NPSSO", e);
    }
  }
  if (!tokens) {
    try {
      const codigo = await exchangeNpssoForAccessCode(PSN_NPSSO.value().trim());
      tokens = await exchangeAccessCodeForAuthTokens(codigo);
    } catch (e) {
      logger.error("O NPSSO do robô não funcionou (vencido ou errado)", e);
      throw new HttpsError(
        "unavailable",
        "O servidor da PlayStation está em manutenção. Tente de novo mais tarde.",
      );
    }
    await anotarQuemEOrobo(tokens);
  }
  await ROBO.set(
    {
      accessToken: tokens.accessToken,
      accessExp: agora + (tokens.expiresIn ?? 3600) * 1000,
      refreshToken: tokens.refreshToken ?? salvo.refreshToken ?? null,
      refreshExp: tokens.refreshTokenExpiresIn
        ? agora + tokens.refreshTokenExpiresIn * 1000
        : salvo.refreshExp ?? 0,
      atualizado: agora,
    },
    { merge: true },
  );
  return { accessToken: tokens.accessToken };
}

/** Escreve no registro (Firebase > Functions > Registros) qual conta PSN o robô está usando. */
async function anotarQuemEOrobo(tokens) {
  try {
    const r = await getProfileFromAccountId({ accessToken: tokens.accessToken }, "me");
    logger.info(`Robô da PSN conectado como: ${r?.onlineId ?? r?.profile?.onlineId ?? "?"}`);
  } catch (e) {
    logger.info("Robô da PSN conectado (não deu para ler o nick dele)");
  }
}

// ---------------------------------------------------------------------------
// Ajudantes
// ---------------------------------------------------------------------------

function exigirConta(req) {
  if (!req.auth?.uid) {
    throw new HttpsError("unauthenticated", "Entre na sua conta do AbyssHub primeiro.");
  }
  return req.auth.uid;
}

/** Perfil público pelo nick (traz accountId e o "Sobre"). */
async function perfilPorNick(acesso, nick) {
  try {
    const r = await getProfileFromUserName(acesso, nick);
    const p = r?.profile ?? r;
    if (!p?.accountId) throw new Error("sem accountId");
    return p;
  } catch (e) {
    logger.info("Perfil não encontrado", nick, String(e?.message ?? e));
    throw new HttpsError(
      "not-found",
      "Não achei esse ID online da PSN. Confira se digitou igualzinho (maiúsculas e minúsculas contam).",
    );
  }
}

/** "PT243H18M48S" -> minutos */
function minutos(duracao) {
  const m = /^PT(?:(\d+)H)?(?:(\d+)M)?(?:(\d+(?:\.\d+)?)S)?$/.exec(duracao ?? "");
  if (!m) return 0;
  return Number(m[1] ?? 0) * 60 + Number(m[2] ?? 0) + Math.floor(Number(m[3] ?? 0) / 60);
}

function codigoNovo() {
  const letras = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // sem 0/O e 1/I para não confundir
  let c = "";
  for (let i = 0; i < 6; i++) c += letras[Math.floor(Math.random() * letras.length)];
  return `ABYSS-${c}`;
}

function erroPrivado(e) {
  const t = String(e?.message ?? e).toLowerCase();
  return t.includes("access") || t.includes("permit") || t.includes("privacy") || t.includes("403");
}

// ---------------------------------------------------------------------------
// Funções que o app chama
// ---------------------------------------------------------------------------

/** 1) A pessoa manda o ID online; o servidor acha a conta e gera o código para o "Sobre". */
export const psnIniciar = onCall(opcoes, async (req) => {
  const uid = exigirConta(req);
  const nick = String(req.data?.nick ?? "").trim();
  if (!/^[A-Za-z0-9_-]{3,16}$/.test(nick)) {
    throw new HttpsError("invalid-argument", "O ID online da PSN tem de 3 a 16 letras, números, - ou _.");
  }
  const ref = db.collection(CONEXOES).doc(uid);
  const antes = (await ref.get()).data();
  if (antes?.pedidoEm && Date.now() - antes.pedidoEm < INTERVALO_CODIGO_MS) {
    throw new HttpsError("resource-exhausted", "Espere alguns segundos e tente de novo.");
  }
  const acesso = await acessoRobo();
  const p = await perfilPorNick(acesso, nick);
  const codigo = antes?.nick?.toLowerCase() === nick.toLowerCase() && antes?.codigo ? antes.codigo : codigoNovo();
  await ref.set({
    uid,
    nick: p.onlineId ?? nick,
    accountId: String(p.accountId),
    codigo,
    verificado: false,
    pedidoEm: Date.now(),
  });
  return { nick: p.onlineId ?? nick, codigo };
});

/** 2) Confere se o código está no "Sobre" do perfil. */
export const psnVerificar = onCall(opcoes, async (req) => {
  const uid = exigirConta(req);
  const ref = db.collection(CONEXOES).doc(uid);
  const c = (await ref.get()).data();
  if (!c?.codigo) throw new HttpsError("failed-precondition", "Gere um código primeiro.");
  if (c.verificado) return { nick: c.nick, verificado: true };
  if (c.verificadoEm && Date.now() - c.verificadoEm < 5_000) {
    throw new HttpsError("resource-exhausted", "Espere alguns segundos e tente de novo.");
  }
  await ref.update({ verificadoEm: Date.now() });
  const acesso = await acessoRobo();
  const p = await perfilPorNick(acesso, c.nick);
  if (String(p.accountId) !== c.accountId) {
    throw new HttpsError("failed-precondition", "Essa conta da PSN mudou. Gere um código novo.");
  }
  const sobre = String(p.aboutMe ?? "").toUpperCase();
  if (!sobre.includes(c.codigo)) {
    throw new HttpsError(
      "failed-precondition",
      `Não achei o código ${c.codigo} no "Sobre" do seu perfil. Salve o perfil e espere uns segundos antes de verificar.`,
    );
  }
  await ref.update({ verificado: true, verificadoEm: Date.now() });
  return { nick: c.nick, verificado: true };
});

/** 3) Jogos e horas da pessoa (no máximo uma leitura na PSN a cada 10 minutos). */
export const psnJogos = onCall(opcoes, async (req) => {
  const uid = exigirConta(req);
  const ref = db.collection(CONEXOES).doc(uid);
  const c = (await ref.get()).data();
  if (!c?.verificado) throw new HttpsError("failed-precondition", "PlayStation não conectada.");
  const agora = Date.now();
  if (c.jogos && c.jogosEm && agora - c.jogosEm < INTERVALO_JOGOS_MS) {
    return { id: c.accountId, nick: c.nick, jogos: c.jogos };
  }
  const acesso = await acessoRobo();
  const jogos = [];
  try {
    let offset = 0;
    for (let pagina = 0; pagina < 5; pagina++) {
      const r = await getUserPlayedGames(acesso, c.accountId, {
        categories: "ps4_game,ps5_native_game",
        limit: 200,
        offset,
      });
      for (const t of r?.titles ?? []) {
        const min = minutos(t.playDuration);
        if (min <= 0 || !t.titleId) continue;
        jogos.push({
          appid: String(t.titleId),
          nome: String(t.localizedName ?? t.name ?? ""),
          min,
          ultima: t.lastPlayedDateTime ? Math.floor(Date.parse(t.lastPlayedDateTime) / 1000) : 0,
          imagem: String(t.localizedImageUrl ?? t.imageUrl ?? ""),
        });
      }
      if (r?.nextOffset == null || r.nextOffset <= offset) break;
      offset = r.nextOffset;
    }
  } catch (e) {
    if (e instanceof HttpsError) throw e;
    logger.warn("Erro lendo os jogos da PSN", c.nick, String(e?.message ?? e));
    if (erroPrivado(e)) {
      throw new HttpsError(
        "permission-denied",
        "Seus jogos da PSN estão privados. Na privacidade da PSN, deixe os jogos como \"Qualquer pessoa\".",
      );
    }
    throw new HttpsError("unavailable", "A PSN não respondeu agora. Tente de novo daqui a pouco.");
  }
  await ref.update({ jogos, jogosEm: agora });
  return { id: c.accountId, nick: c.nick, jogos };
});

/** 4) Esquece a conta da PSN dessa pessoa. */
export const psnDesconectar = onCall(opcoes, async (req) => {
  const uid = exigirConta(req);
  await db.collection(CONEXOES).doc(uid).delete();
  return { ok: true };
});
