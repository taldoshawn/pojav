# Validação da alpha 0.1.0

APK: commit `167ff1a1375b361139ed4790b4372e465a712e78`, código Android 1002.
[Release com downloads](https://github.com/taldoshawn/pojav/releases/tag/v0.1.0-alpha.1).

| Verificação | Resultado | Evidência |
| --- | --- | --- |
| Compilação ARM64, ARMv7, x86, x86_64 | Aprovada | [Build #2](https://github.com/taldoshawn/pojav/actions/runs/43251468836) |
| 166 testes unitários de protocolo + 10 testes do transporte legado | Aprovados | Mesmo build |
| Assinatura permanente, conteúdo público, SHA-256 dos quatro APKs publicados | Aprovados | [Publicação verificada](https://github.com/taldoshawn/pojav/actions/runs/43309163143), relatórios anexos à release |
| Instalação e abertura da interface x86_64, Android 15/API 35 | Aprovadas | [Smoke test](https://github.com/taldoshawn/pojav/actions/runs/43307924129), artifact `android-startup-evidence` |
| Download verificado dos recursos, carregamento de tabelas e tela inicial do jogo Rust | Aprovados no emulador x86_64 | [Capturas e log](https://github.com/taldoshawn/pojav/actions/runs/43308716656) |
| Mundo local, criação, movimento e interação com blocos | Ainda não validados | Não inferir de um teste de abertura |
| Login Microsoft e servidor online autenticado | Ainda não validados | Requer conta legítima; nenhum token de jogador está no teste |
| FPS, estabilidade térmica e consumo em aparelhos físicos | Não medidos | SwiftShader do emulador não representa uma GPU de celular |
| Gameplay 1.8.9 | Não implementado | O transporte protocolo 47 ainda precisa de integração ao motor |

O teste do launcher verifica instalação, Activity em primeiro plano, processo
persistente e ausência de marcadores de crash, com screenshot e logcat. A opção
`probe_game` gera capturas durante download e tentativa de entrada no jogo;
**o workflow passar não significa que o mundo foi validado**. As capturas precisam
ser inspecionadas e cada etapa deve ter uma evidência própria.

A cobertura de quatro arquiteturas não garante todos os aparelhos. Android mínimo,
GPU, suporte do driver, RAM e espaço disponível continuam sendo requisitos.
