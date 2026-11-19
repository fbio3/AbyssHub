# Mudanças do AbyssHub

## v2.2.0 — PlayStation

- **Jogos da PlayStation (PS4 e PS5)**: em Configurações > PlayStation, digite o seu ID online, coloque o código que o app mostra no "Sobre" do seu perfil e toque em Verificar. Você não precisa dar a senha; o perfil só precisa estar público.
- Os jogos da PSN entram em todas as abas e no Leaderboard, do mesmo jeito que os da Steam: as horas que você já tinha contam em "Tudo", e Hoje, Semana e Mês contam o que você jogar depois de conectar.
- O mesmo jogo no celular, na Steam e na PSN conta como um só.
- Leaderboard: nova opção **PSN** no filtro de plataforma e um símbolo de controle para os jogos da PlayStation.

## v2.1.1

- O Leaderboard volta a mostrar quem ainda não atualizou o app (v2.0 ou antes), junto com quem está na versão nova. Essas pessoas aparecem em Todas e Mobile (as versões antigas não têm Steam nem o período Mês).
- O Leaderboard guarda o último ranking no celular: ao abrir o app (ou depois de uma atualização) ele já aparece na hora, sem ficar vazio carregando, e é trocado pelo novo assim que chega.
- O "atualizado há..." do Leaderboard agora mostra minutos, horas ou dias.

## v2.1.0 — Steam

- **Jogos da Steam**: em Configurações > Steam, conecte a sua conta com uma chave grátis da própria Steam (o app mostra o passo a passo). A chave fica guardada só no seu celular.
- Os jogos da Steam aparecem junto com os do celular, em todas as abas e no Leaderboard. As horas que você já tinha contam em "Tudo"; Hoje, Semana e Mês contam o que você jogar depois de conectar.
- O mesmo jogo no celular e na Steam conta como um só.
- A Steam é atualizada sozinha a cada 10 minutos com o app aberto (e no botão de atualizar).
- Leaderboard: novo filtro de plataforma (**Todas · Mobile · Steam**), em cima dos outros filtros.
- Leaderboard: novo período **Mês** (últimos 30 dias).
- Leaderboard: símbolo no canto de cada jogador mostrando se o jogo é do celular, da Steam (PC) ou dos dois.
- O Leaderboard não precisa mais de índices novos no Firebase.

## v2.0.1

- O app procura versão nova toda vez que é aberto (e também quando volta do segundo plano), além de a cada 6 horas enquanto está aberto.
- Se não tiver internet na hora, tenta de novo sozinho depois de 10 minutos.

## v2.0.0

- Rolagem com impulso em todo o app: ao soltar o dedo a página continua rolando conforme a força do movimento e vai parando aos poucos.
- Barra de rolagem da direita mais grossa, fácil de pegar e arrastar com o dedo.
- Jogos repetidos agora contam como um só: o mesmo jogo instalado de lojas diferentes (ex.: Roblox da Play Store e da Galaxy Store) é juntado pelo nome.
- No Leaderboard, o mesmo jogo de lojas diferentes também aparece junto, para todo mundo.
- Configurações: mudar o nome que aparece no Leaderboard, mesmo entrando com Google.
- Configurações: mudar a foto do Leaderboard (escolher da galeria, usar a foto da conta Google ou ficar sem foto).
- Configurações: conectar uma conta Google a quem entrou com e-mail e senha, e adicionar e-mail e senha a quem entrou com Google.
- A conta mostra com o que você entra (Google, e-mail ou os dois).
- Configurações: a lista "O que conta como jogo" mostra só os 10 primeiros, com o botão "Ver todos os apps" para abrir a lista inteira.
- A lista de apps não repete mais o mesmo jogo de lojas diferentes.
- Seus documentos antigos do Leaderboard (jogos desinstalados ou repetidos) são limpos sozinhos.

## v1.0.0 — primeira versão pública

- Novo ícone do AbyssHub (o explorador olhando o abismo, com o controle dourado), também como ícone adaptável do Android.
- Login com **e-mail e senha**, além do Google: criar conta, entrar e "Esqueci minha senha".
- **Atualização automática** pelo GitHub, igual ao AbyssSub: o app procura versão nova sozinho (a cada 6 horas), mostra o que mudou, baixa e abre o instalador.
- Painel "Nova versão disponível" com a versão atual e a nova, o tamanho, a barra de download e os botões "Agora não" e "Atualizar agora".
- Se o Android pedir permissão para instalar, a instalação continua sozinha quando você volta para o app.
- Nas configurações, o grupo "Atualizações" mostra a versão instalada e tem o botão "Procurar atualização".
- APK gerado e anexado sozinho em cada versão publicada no GitHub (GitHub Actions).
- Textos de erro do login mais claros ("E-mail ou senha incorretos", "Esse e-mail já tem uma conta"...).
- README novo, explicando o app para quem vai baixar.
- Projeto preparado para ficar público no GitHub (licença MIT e arquivos particulares fora do repositório).

---

### Versões de teste (antes da 1.0)

#### v0.3.0 — Leaderboard

- Nova aba **Leaderboard** (com ícone de troféu) para ver quem mais jogou.
- Três jeitos de ver o ranking: por pessoa (com o jogo que ela mais jogou), por pessoa + jogo e por jogo.
- Três períodos: Semana (o padrão), Hoje e Tudo.
- Pódio com os 3 primeiros: foto com anel de ouro, prata e bronze, degraus que sobem e troféu no 1º lugar.
- Classificação do 4º lugar em diante, com foto da pessoa, ícone e nome do jogo e o tempo.
- Sua linha aparece destacada em dourado, escrito "você".
- No topo: sua posição, seu tempo, quem é o líder e quanto falta para a próxima atualização.
- Atualiza sozinho a cada minuto enquanto a aba está aberta; dá para tocar para atualizar na hora.
- Login com Google (o mesmo Firebase do AbyssSub); só aparece no ranking quem está logado.
- Seu tempo de jogo é enviado a cada minuto, só quando mudou, e só dos apps que contam como jogo.
- Fotos das pessoas vêm da conta Google.
- Ícones de verdade dos jogos em todas as telas (antes era só a letra inicial).
- Ícones dos jogos ficam salvos no Firebase, para quem não tem o jogo instalado também ver.
- Fotos e ícones ficam guardados no celular, para não baixar de novo.
- Modo "Por jogo" com a lista de jogos para escolher deslizando para o lado.
- Configurações ganharam "Conta e Leaderboard": entrar, sair, ou sair e tirar seus dados do ranking.
- Botão de criar índice no Firebase quando o banco pede (aparece no próprio erro).
- O APK agora é montado pelo Gradle, igual ao AbyssSub (pasta android/).
- Novos arquivos: firestore.rules, firestore.indexes.json, CONFIGURAR-FIREBASE.md e firebase-sha1.bat.

#### v0.2.0 — visual do conjunto Abyss

- Visual igual ao do AbyssSub: topo em degradê azul com cantos arredondados, cartões arredondados e destaques dourados.
- Tema claro e escuro com as mesmas cores do AbyssSub; segue o tema do celular ou dá para escolher nas configurações.
- Troca de tema suave, com as cores mudando aos poucos.
- Fonte Inter, a mesma cara do AbyssSub.
- Ícone novo do AbyssHub (a arte do Abyss com um controle dourado) no app e na tela inicial do celular.
- Barra de abas embaixo: Início, Jogos e Histórico, com a pílula da aba ativa acendendo.
- Início: rosca "Por jogo" que se desenha ao abrir, gráfico dos últimos 7 dias com barras que crescem e lista dos mais jogados.
- Jogos: ranking com posição (#1 em dourado), tempo e barra de cada jogo.
- Histórico: sessões agrupadas por dia (Hoje, Ontem, Sex, 14 de nov...).
- Detalhe do jogo abre deslizando da direita, com gráfico próprio, interruptor "Contar como jogo" e lista de sessões.
- Seletor de período (Hoje, 7 dias, 30 dias, Tudo) com a bolinha deslizando.
- Número grande do topo conta até o valor ao abrir a tela.
- Comparação com o período anterior (ex.: +25%) e média por dia no topo.
- Selo "Jogando agora" com bolinha pulsando quando um jogo está aberto.
- Brilho do topo flutuando devagar, como no AbyssSub.
- Cartões entram subindo um depois do outro; listas aparecem em sequência.
- Botão dourado flutuante para atualizar, que gira enquanto atualiza.
- Aviso na parte de baixo quando a atualização termina (ou dá erro).
- Configurações num painel que sobe de baixo: tema, animações, mostrar todos os apps e o que conta como jogo.
- Interruptores dourados animados, iguais aos do AbyssSub.
- Toque nos itens com efeito de afundar; engrenagem gira um pouco ao tocar.
- Opção para desligar as animações (economiza bateria).
- Botão voltar do Android fecha o painel e o detalhe.
- O topo aparece atrás da barra de status do celular.

#### v0.1.0 — primeira versão

- Lê o histórico de uso do Android (UsageStatsManager) para descobrir quando cada app foi aberto e fechado.
- Detecta sozinho quais apps são jogos (categoria "Jogo" do Android).
- Junta as aberturas em sessões de jogo, entendendo troca de app, tela desligada e jogos com várias telas.
- Salva as sessões no próprio celular, para o tempo não sumir quando o Android apaga o histórico antigo.
- Sincroniza sozinho ao abrir o app e a cada 30 segundos enquanto ele está aberto.
- Tela inicial com o tempo total e a lista de jogos por tempo: hoje, 7 dias, 30 dias ou tudo.
- Tela de cada jogo com total, número de sessões, média, sessão mais longa, gráfico dos últimos 7 dias e lista de sessões.
- Mostra "Jogando agora" quando um jogo ainda está aberto.
- Ajustes para marcar ou desmarcar apps como jogo e para mostrar também apps que não são jogos.
- Tela que explica e abre a permissão "Acesso ao uso".
- Tema escuro Abyss (azul profundo e dourado).
- Modo de teste no PC (`cargo run`) com jogos de exemplo.
