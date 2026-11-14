# Comparação dos controles

Referência pública consultada: PojavLauncherTeam/PojavLauncher, branch
`v3_openjdk`, commit `b12ad048157b3aa255d078c235dd4571e1900309`.
Arquivos: `customcontrols/ControlData.java`, `buttons/ControlButton.java` e
`mouse/Touchpad.java`, em `app_pojavlauncher/src/main/java/net/kdt/pojavlaunch/`.
A referência usa LGPL-3.0; nenhum código Java foi incorporado nesta revisão.

O modelo de controles da referência inclui até quatro teclas por botão,
opacidade, tamanho, posição, alternância, visibilidade em menus/jogo,
deslizamento entre botões e passagem de movimento para a câmera.
O editor Rust possui campos correspondentes; isso não comprova equivalência
comportamental ou qualidade equivalente no aparelho.

## Verificação em andamento

- Teclas compartilhadas por dois dedos: emitir apenas uma transição de pressionar;
  soltar apenas após o último dedo. Corrigida emissão duplicada no código Rust.
- Alternância combinada com toque: ativar a trava de uma tecla já segurada não
  deve gerar outro pressionamento; retirar um dedo não deve liberar a trava.
- Cancelamento: liberar todas as teclas quando o gesto for cancelado.
- Persistência: conferir nome, tamanho, posição e comandos após reiniciar o APK.
- Mouse virtual: conferir clique, arrasto, sensibilidade e cursor personalizado.
- Teclado: conferir chat, caracteres acentuados e teclado físico.

O workflow Android executa os testes de controles customizados no host antes
finalizar a variante ARM64. Esses testes verificam lógica de entrada; testes
no APK e em aparelhos continuam necessários para avaliar latência e ergonomia.
