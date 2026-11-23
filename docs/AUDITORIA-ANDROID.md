# Auditoria do port Android do WordCraft

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
