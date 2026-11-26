# Auditoria do port Android do WordCraft

## Retomada em 26 de novembro de 2026

Use o checkout de `botvit10/wordcraft-android` na branch `ci/android-arm64-baseline`. O último commit validado pelo GitHub Actions é `5e817fc`. O APK local corresponde a `593749d`. Entre esses commits só mudaram o workflow e os testes de dispositivo. O motor continua em **WordCraft 0.5.0**, fixado em `storytold/wordcraft@2ffba93b5189410a415970be179dcf481092d756`. A atualização depende de autorização explícita do usuário.

O próximo passo é obter o resultado deste APK no tablet. O usuário já confirmou na compilação anterior que DOCX abre, que digitar espaço mantém o teclado aberto e que **Colar** no menu do WordCraft funciona. A queixa de colagem restante veio do **histórico do teclado**. [Resultados e pendências](#validação-da-integração-em-android-15) · [Prompt para o próximo chat](PROMPT-CONTINUACAO.txt)

| Marco | O que mudou |
|---|---|
| `4ee29b9` | Referência inicial de compilação ARM64 local e no Actions. |
| `e2d732f` | Filtros distintos no seletor interno, correção de foco ao digitar espaço e integração do comando Colar com a área de transferência Android. O seletor de imagens ainda dependia da listagem por caminho. |
| `d40e7dc` a `593749d` | Seletor Android para inserir ou trocar imagens, conexão com o teclado para receber texto e imagens e menu de correção ajustado à área visível em paisagem. |
| `5e817fc` | Validação automatizada no Android 15. O teste abre a lista de Arquivos para tocar no nome do documento. A grade também oferece um botão separado de pré-visualização. |

## Base reproduzida

O commit `5f0ed9162c19a5b4904545ef2667885fe83d84d2` de `botvit10/wordcraft-android` coincide com o HEAD consultado de `kanaaka/wordcraft-android`. O changelog identifica a distribuição do port como v0.1.2. O código do editor vem de `storytold/wordcraft` 0.5.0, commit `2ffba93b5189410a415970be179dcf481092d756`. São duas versões distintas. [Motor no commit auditado](https://github.com/storytold/wordcraft/tree/2ffba93b5189410a415970be179dcf481092d756)

| Componente | Conteúdo e efeito |
|---|---|
| README, CHANGELOG, docs/PORT-NOTES | Receita do mantenedor, histórico de APKs e testes anteriores em tablet e emulador. |
| patches/0001 | Altera dois arquivos upstream. Cargo.toml aponta para winit local. A UI ganha o serviço opcional new_window, ligado ao comando de segunda janela. |
| apps/wordcraft-android | Crate cdylib, ponto de entrada android_main, serviços JNI e egui, preferências atômicas, seletor de arquivos, adaptação de modificadores e gestos. res contém temas e ícones por densidade. |
| vendor/winit | winit 0.30.13 com encaminhamento de roda e hover no backend Android. A resolução local faz parte da funcionalidade de mouse. |
| scripts originais | android-env.sh, build-apk.sh e verify-apk.sh pressupõem Windows/MSYS, diretórios do mantenedor e um keystore local. A nova receita Linux usa o SDK instalado pelo runner. |
| patches/0002 e scripts de CI | O delta de Cargo.lock registra o membro Android e a origem local do winit. Todas as versões das dependências upstream são preservadas. O CI monta a árvore em diretório novo e verifica o lockfile após o empacotamento. |
| Licenças e atribuição | LICENSE-MIT, LICENSE-APACHE, ATTRIBUTION.md e vendor/README.md documentam os respectivos componentes. |

## Recursos e limites

| Área | Evidência e limite |
|---|---|
| Edição e formatos | O shell instancia WordApp com o motor, modelo de documento e UI originais. DOCX, tabelas, imagens, formatação e exportação PDF usam os crates Rust do desktop. Os testes de biblioteca exercitam os formatos no Linux. O percurso completo pela tela Android ainda requer ensaio. |
| Execução e armazenamento | NativeActivity carrega libwordcraft.so, com eframe/egui e renderização nativa. Os arquivos do usuário, preferências e fontes são locais. O manifesto do port dispensa a permissão INTERNET. O primeiro build precisa baixar ferramentas e fontes de dependências. |
| Abrir e salvar | O seletor egui devolve caminhos para abrir, salvar como, exportar e inserir imagens. A pasta do app funciona com armazenamento privado. Pastas compartilhadas dependem das permissões da versão do Android. Faltam Storage Access Framework, content URIs e integração com provedores de documentos. MANAGE_EXTERNAL_STORAGE exige concessão manual e revisão de elegibilidade na Play Store. |
| Entrada | O código distingue seleção, rolagem com dois dedos e pinça. O winit adaptado encaminha mouse e trackpad. O port trata Ctrl físico/lógico e modificadores por evento. Faltam pressão e inclinação de caneta. Digitação com IME, acentos, composição, seleção e teclado cobrindo o cursor exigem validação atual em aparelho. |
| Celular e tablet | O port preserva a faixa de opções do desktop, temas fullscreen, tratamento de redimensionamento e segunda Activity. A documentação registra limites de insets da IME/barra do sistema no Android 15 e de edge-to-edge no Android 16. A usabilidade da faixa de opções em largura compacta continua pendente. |
| Serviços do sistema | O shell conecta file_dialog e new_window. Impressão pelo sistema, colagem de imagem de outros apps e saída de áudio permanecem ausentes. Exportar PDF pelo motor é uma operação distinta da impressão pelo Android. |

## Reprodução em Linux x86_64

A base usa Rust 1.99.0, cargo-apk 0.10.0, JDK 21, SDK Platform 35, Build Tools 35.0.0 e NDK r27c (27.2.12479018). O linker do NDK usa a API mínima 26. Cargo.lock fixa eframe e egui 0.36.2, wgpu 30.0.1, jni 0.22.4 e android-activity 0.6.1. O workflow fixa Temurin 21.0.13+9 e as actions por SHA. A imagem ubuntu-24.04 recebe atualizações do GitHub.

Com Git, Bash, Python 3, unzip, rustup, JDK, SDK command-line tools e platform-tools instalados, exporte ANDROID_HOME e coloque sdkmanager e keytool no PATH. A etapa de teste Linux usa o backend X11. Na raiz do port, execute com um diretório de saída novo:

```bash
rustup toolchain install 1.99.0 --profile minimal --target aarch64-linux-android
cargo +1.99.0 install cargo-apk --version '=0.10.0' --locked
sdkmanager 'platforms;android-35' 'build-tools;35.0.0' 'ndk;27.2.12479018'
scripts/ci-arm64.sh /tmp/wordcraft-build
(cd /tmp/wordcraft-build/artifact && sha256sum -c SHA256SUMS)
```

O script executa os testes do port, compila o APK release para aarch64, confere assinatura, alinhamento, pacote, ABI, Activity de lançamento e símbolos da biblioteca embarcada. Para repetir os testes adicionais do motor, execute na árvore montada:

```bash
cd /tmp/wordcraft-build/source
cargo +1.99.0 test --locked --lib -p wordcraft-docx -p wordcraft-pdf
```

Push, pull request e disparo manual acionam o workflow. Uma execução bem-sucedida publica o artifact `wordcraft-arm64-test` por 14 dias com APK, SHA-256 e avisos. A chave RSA de teste é criada fora do checkout, com acesso privado ao arquivo. Cada execução usa um certificado distinto. Para instalar sobre uma versão com outro certificado é preciso desinstalar a anterior, o que pode apagar os dados privados do app. Use um perfil ou aparelho de teste. [Workflow](../.github/workflows/android-arm64.yml)

## Redistribuição

WordCraft e o port oferecem MIT ou Apache-2.0. Preserve autoria, atribuições e os textos aplicáveis. Ao distribuir sob Apache-2.0, indique os arquivos modificados e preserve os avisos aplicáveis de NOTICE. O winit vendorizado usa Apache-2.0. O port é uma modificação não oficial. As marcas ArtCraft têm termos próprios. [Atribuição do port](../ATTRIBUTION.md) · [NOTICE upstream](https://github.com/storytold/wordcraft/blob/2ffba93b5189410a415970be179dcf481092d756/NOTICE)

As fontes Inter, JetBrains Mono, Source Sans 3 e Source Serif 4 têm termos OFL. As fontes do egui incluem outros termos, entre eles Ubuntu Font Licence. O inventário do grafo ARM64 inclui também BSD, ISC, Zlib e Unicode. O artifact acompanha o APK com os textos do projeto, NOTICE, atribuições e os avisos encontrados nos crates. Distribua esse conjunto no teste. Antes de uma distribuição pública do APK avulso, confira os direitos de cada recurso e disponibilize os avisos também aos usuários que só recebam o APK.

## Evidência e sequência técnica

A compilação de referência na nuvem produziu um APK ARM64 com assinaturas v2/v3, pacote ai.storyteller.wordcraft e símbolos android_main e ANativeActivity_onCreate. Os sete testes do port passaram no Linux. A auditoria executou também 60 testes de wordcraft-docx e 22 de wordcraft-pdf, todos aprovados. A execução interativa desta versão em dispositivo está pendente.

1. Instalar em celular e tablet ARM64 de teste. Em modo avião, abrir DOCX com tabela e imagem, editar, salvar, reabrir e exportar PDF. Conferir conteúdo e paginação. Repetir após rotação e retomada do app.
2. Medir largura útil, IME e insets em retrato, paisagem e tela dividida. Testar teclado virtual em português, atalhos físicos, mouse, toque e troca de janela. Usar os resultados para definir a faixa compacta e os alvos de toque.
3. Integrar o seletor Android com ACTION_OPEN_DOCUMENT e ACTION_CREATE_DOCUMENT, persistência das permissões de URI e testes com provedores locais. Preservar acesso privado offline.
4. Após autorização para atualizar o motor, comparar os commits com o pin atual, revisar as interfaces Services e DOCX/PDF, reavaliar o patch do winit e repetir a mesma matriz de regressão antes de trocar a versão base.

## Teste em aparelho após a base 4ee29b9

O primeiro teste em aparelho confirmou a abertura de DOCX. Em Inserir → Imagens, a pasta Download mostrou documentos e ocultou as imagens. O motor 0.5.0 envia `insertPicture` para a UI, que pede `Open { purpose: "picture" }`. Arquivo → Abrir pede `"document"`. Na base auditada, o caso `"picture"` já incluía PNG e JPEG. O seletor usava `read_dir`, mantinha as pastas visíveis e comparava a última extensão sem distinguir maiúsculas e minúsculas. O código da base não permite atribuir a listagem observada a uma extensão ausente.

O novo seletor mantém o tipo do pedido até desenhar a lista. O modo “Insert or change picture” abre Download se a pasta estiver acessível, mostra os formatos aceitos e mantém o filtro de imagem ao trocar de pasta. O log privado `wordcraft.log` registra a finalidade e as extensões ao abrir o seletor. Permissões de armazenamento do Android ainda determinam quais entradas chegam a `read_dir`.

Espaço e Enter podem ativar um clique de teclado sobre o canvas. No egui 0.36.2, pedir foco novamente interrompe a composição. O egui-winit desativa e reativa a IME, e o winit Android traduz isso em ocultar e mostrar o teclado. O canvas agora pede foco somente quando precisa adquiri-lo.

O backend egui-winit mantém texto em memória no Android. A nova integração dos comandos do editor usa `ClipboardManager` para copiar e recortar texto e para ler a primeira entrada em Colar. Imagens de outro app precisam chegar como uma URI `content://` com MIME de imagem e permissão de leitura. A leitura aceita até 16 MiB de dados codificados. Copiar uma imagem do documento para fora ainda requer publicar os bytes por uma URI com permissão temporária.

No aparelho de teste:

1. Coloque um PNG, um JPEG e um DOCX na mesma pasta Download. Em Inserir → Imagens, confira o título e a lista. Insira cada imagem em uma posição distinta do documento. Salve e reabra pelo menu Arquivo → Abrir. Confira as duas imagens e o filtro de documentos.
2. Com o teclado virtual em português, digite “ação com espaço”, repita espaços, aceite uma sugestão e pressione Enter. Confira o texto, o cursor e se o teclado permanece aberto.
3. Copie texto com acentos entre WordCraft e outro app nos dois sentidos. Teste Recortar e Colar pelos menus. Copie uma foto como imagem em outro app e cole no documento. Salve e reabra para conferir a foto.

Se a listagem do aparelho divergir, registre o título do seletor, o caminho exibido, a versão do Android e a permissão concedida. Repita com os mesmos arquivos em App folder para distinguir acesso a Download de filtragem.

## Retorno do aparelho sobre e2d732f

O usuário confirmou que digitar espaço mantém o teclado aberto e que o comando Colar do WordCraft funciona. O item escolhido no histórico do teclado não chegava ao editor. A captura de Inserir → Imagens mostrou o modo de imagem ativo em DCIM/Camera, com a lista vazia. No menu de correção, o teclado cobria as opções inferiores.

A Activity abre o seletor Android com `ACTION_OPEN_DOCUMENT` e `image/*`. Lê a URI com a concessão da seleção e remove a cópia de cache depois da inserção. PNG e JPEG conservam os bytes originais. Fotos em outros formatos reconhecidos pelo Android podem ser convertidas em PNG. A conexão de entrada aceita texto em bloco e imagens oferecidos pelo teclado. A área disponível acima da IME passa a orientar o posicionamento e a rolagem do menu de contexto.

No tablet, selecione uma foto de Câmera e um PNG de Downloads, salve e reabra o DOCX. No histórico do teclado, escolha uma frase com acentos anterior ao item mais recente, depois uma imagem se o teclado oferecer essa opção. Em paisagem, mantenha o teclado aberto, pressione uma palavra sublinhada e role até as opções inferiores.

### Validação da integração em Android 15

No emulador de tablet em paisagem, o APK ARM64 com o código de aplicação de `593749d` abriu **Inserir → Imagens** na tela Recentes do seletor Android. Um PNG de Download e um JPEG de DCIM/Camera apareceram no filtro padrão. A seleção das duas miniaturas produziu duas imagens distintas no mesmo documento.

Com o teclado de teste em outro processo, o editor recebeu “histórico ação” enquanto a área de transferência continha “item atual”. O mesmo teclado enviou uma terceira imagem por `commitContent`, que apareceu na página. Com a IME visível, manter o dedo sobre uma palavra sublinhada abriu o menu de correção acima do teclado. A última opção ficou visível ao soltar o dedo. O teclado e a galeria do aparelho do usuário ainda exigem a repetição desses gestos.

Os 160 testes no Linux passaram. A suíte inclui 12 testes do port, 60 de DOCX, 66 de formatos e 22 de PDF. O teste de regressão salva o PNG e o JPEG no mesmo DOCX, reabre o documento com o filtro de Arquivo → Abrir e confere as duas mídias.

No teste local de **Arquivo → Abrir → Procurar**, o seletor de documentos abriu `WordCraft-abertura.docx` na pasta do app. A página exibiu o texto e as duas imagens do arquivo de amostra. Em outra tentativa, o emulador local sem aceleração de CPU registrou um ANR de entrada após `Ctrl+S` em um documento novo. A causa permanece em aberto. A gravação seguida de reabertura desse documento específico continua pendente no tablet.

O GitHub Actions passou no commit `5e817fc`. Compilou e assinou o APK ARM64, verificou a assinatura, o alinhamento e a ABI, passou nos 12 testes do port e executou o APK no emulador Android 15. A instrumentação selecionou PNG e JPEG pelo aplicativo Arquivos, comparou os bytes e testou o cancelamento. Um serviço de teclado separado enviou a entrada histórica de texto, que foi recuperada do documento pelo comando de cópia, e uma imagem, cuja renderização foi conferida na tela. [Execução 45383617143](https://github.com/botvit10/wordcraft-android/actions/runs/45383617143) · [Artifact com APK, SHA-256 e licenças](https://github.com/botvit10/wordcraft-android/actions/runs/45383617143/artifacts/14914923914)

**APK local.** Neste workspace, o arquivo é `/workspace/wordcraft-test-593749d/wordcraft-arm64-test.apk`. SHA-256 `17fd19f2b97aeff48c8773bc63f62820806cff25ac560af0b46cecbc26207155`. A pasta contém também as capturas do seletor e da correção ortográfica. O APK local conserva o certificado do APK local anterior. O Actions gera uma chave de teste por execução e inclui o checksum correspondente no artifact, com expiração prevista para 8 de dezembro de 2026.

**Onde investigar se o tablet divergir.**

- **Imagens.** Em `apps/wordcraft-android/java/WordCraftActivity.java`, confira `pickPicture` e `onActivityResult`. O pedido usa `ACTION_OPEN_DOCUMENT`, `CATEGORY_OPENABLE` e `image/*`. O provedor devolve uma URI que a Activity lê para o cache. Em `apps/wordcraft-android/src/activity.rs`, o resultado segue para o comando do editor. Arquivo → Abrir continua em `file_picker.rs`.
- **Colagem.** A conexão de entrada na mesma Activity recebe texto do teclado e imagens por `commitContent`. O código Rust entrega os eventos ao editor. O caminho do comando Colar do próprio WordCraft está em `apps/wordcraft-android/src/clipboard.rs`.
- **Correção ortográfica.** A Activity mede a altura visível com a IME aberta. `apps/wordcraft-android/src/lib.rs` aplica o espaço ocupado pelo teclado. O ajuste do menu do canvas está em `patches/0001-wordcraft-android-port.patch`.
- **Reprodução.** O workflow `.github/workflows/android-arm64.yml` usa `scripts/ci-arm64.sh`, `scripts/package-activity.sh` e `scripts/test-device.sh`. As verificações do seletor e do teclado estão em `tests/android/DeviceTest.java` e `tests/android/TestKeyboard.java`.

**Reteste no tablet, nesta ordem.**

1. Em Inserir → Imagens, selecione o PNG de Downloads e o JPEG de Câmera. Coloque o cursor em outra posição entre as inserções para conservar as duas imagens. Salve pelo menu e reabra em Arquivo → Abrir. Confira as duas imagens.
2. No histórico do teclado habitual, escolha uma frase acentuada anterior ao último item copiado. Confira o texto exato e tente uma imagem, se o teclado a oferecer.
3. Em paisagem, mantenha o teclado aberto, pressione uma palavra sublinhada e toque em uma opção inferior do menu de correção. Confira também se digitar espaço mantém o teclado estável.

Se algo falhar, registre qual APK foi instalado, modelo e versão do Android, nome do teclado, gesto e resultado observado. Para imagens, inclua a origem do arquivo e a tela do seletor. Para travamento ao salvar, diferencie menu, botão e atalho físico.
