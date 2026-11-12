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
| Criação e entrada em mundo plano, terreno, criaturas, HUD e controles exibidos | Confirmadas no emulador x86_64 | [Capturas do mundo](https://github.com/taldoshawn/pojav/actions/runs/43310247752) |
| Movimento, quebra/colocação de blocos e todos os tipos de mapa | Ainda não validados | O gesto enviado não basta para comprovar interação |
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

## Testes posteriores à alpha publicada

- [Build #3](https://github.com/taldoshawn/pojav/actions/runs/43319209497):
  quatro arquiteturas compiladas com múltiplos perfis locais. As capturas do
  [teste da interface](https://github.com/taldoshawn/pojav/actions/runs/43321841597)
  mostram PojavOne e PojavTwo, com PojavOne selecionado após reiniciar. A leitura
  automatizada dos arquivos falhou na transição de privilégios do adbd; isso não
  é um teste integral aprovado. O harness passou a verificar o UID após reconectar.
- [Diagnóstico de servidores](https://github.com/taldoshawn/pojav/actions/runs/43322674531):
  status acessível no Mush e Hypixel. No início do login, Mush respondeu pedindo
  validação em `mush.com.br/validar`, tanto com o endereço original quanto com o
  destino SRV no protocolo moderno. Não foi realizada validação, registro de conta
  ou entrada em partida. Não há evidência para alterar o hostname do cliente.
- [Build #4](https://github.com/taldoshawn/pojav/actions/runs/43322601262):
  inclui opções do contador nativo de FPS em Settings > Controls, canto da tela e
  mínimo/1% low. Compilação e teste visual ainda pendentes na redação deste registro.
  Essas mudanças não estão no APK da alpha.1 acima.
