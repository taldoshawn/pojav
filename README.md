# Pojav Rust para Android

Cliente nativo em Rust baseado no [Horizon Android](https://github.com/GoyDevv/Horizon-Android), com renderização Vulkan/OpenGL ES e multiplayer Java.
O jogo não usa uma JVM no celular. Ferramentas Java são usadas apenas na compilação e preparação de recursos.

## APK

As compilações ficam em **Actions → Android APK → Artifacts**. Baixe o arquivo da arquitetura do seu aparelho e extraia o APK do ZIP.
ARM64 é a arquitetura dos celulares atuais. As variantes ARMv7 e x86 de 32 bits são experimentais.

- ARM64 / x86_64: Android 9 (API 28) ou superior.
- ARMv7 / x86: Android 8.1 (API 27) ou superior.
- GPU com OpenGL ES 3 ou Vulkan compatível; memória e drivers também limitam o funcionamento.
- Na primeira abertura, o aplicativo baixa e verifica os recursos oficiais necessários.
- Servidores autenticados precisam de uma conta que possua Minecraft Java.

## Compatibilidade atual

O backend de jogo existente tem como alvo **26.3, protocolo 777**. A nova camada de rede **1.8–1.8.9, protocolo 47**, ainda é experimental e **não está integrada ao jogo**. Não há suporte universal a versões, servidores ou aparelhos, nem promessa de FPS.

A preparação do APK e a compatibilidade de jogo são verificações distintas. Consulte [o estado do protocolo legado](docs/legacy-protocol.md) e [a compilação deste fork](docs/pojav-build.md).

## Origem e licenças

Snapshot de GoyDevv/Horizon-Android no commit `ae4c85dc5b31da2078ee1e1009a53ced7d7ed1ba`, com alterações locais. O histórico Git original não está incluído neste snapshot.

Créditos: Ludovic Marie, colaboradores do Horizon e mantenedores do port Android. Licença [MIT](LICENSE), com [avisos de terceiros](THIRD_PARTY_NOTICES.md). [README original](docs/upstream-README.md).
Este fork não é o PojavLauncher oficial e não é afiliado à Mojang ou Microsoft. JARs, mundos, contas, chaves e recursos originais do Minecraft não são enviados para este repositório.
