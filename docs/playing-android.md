# Jogar, perfis e mapas no Android

## APK e primeira abertura

Baixe a [release assinada](https://github.com/taldoshawn/pojav/releases/tag/v0.1.0-alpha.1)
adequada à arquitetura do aparelho. ARM64 é a opção usual para celulares atuais.
Toque em **Download Minecraft 26.3**, aguarde a verificação dos recursos e toque em
**Play Minecraft 26.3**. O teste em instalação limpa baixou cerca de 504 MiB.
A tela de boas-vindas do jogo tem um botão **Continue**; depois use **Singleplayer**
para criar ou abrir um mundo.

A criação e entrada em mundo plano foram observadas no emulador. Outros tipos
de mundo, importações arbitrárias e desempenho em celulares não estão certificados.
Veja [as verificações realizadas](android-validation.md).

## Perfis locais e contas autenticadas

O botão da conta fica no canto superior direito do launcher. Escolha o perfil
local para jogar sozinho, em LAN ou em servidores que aceitem modo offline.
Use um nome de 3 a 16 caracteres: letras ASCII, números ou `_`.

O código a partir do commit `98ec02d` permite salvar vários nomes e alternar
entre eles no seletor de contas, sem apagar as contas Microsoft. Essa melhoria
está no build em preparação; a alpha 1 publicada ainda tem o campo de nome local
original. Um perfil local não é uma conta Microsoft nem autentica em servidores
que exigem uma conta com licença. Selecionar um perfil local também não registra
automaticamente uma conta em servidores que tenham cadastro próprio.

O [Hypixel exige uma conta de Minecraft Java](https://support.hypixel.net/hc/en-us/articles/360019495360-How-to-Join-the-Hypixel-Server).
O [Mush recomenda a 1.8.9](https://forum.mush.com.br/topic/327396/ajuda), que ainda
não é um backend jogável deste APK. Nem uma listagem de versões aceitas pelo
servidor nem uma resposta de ping comprovam compatibilidade de gameplay.

## Mapas existentes

A base Android oferece importação de pasta e de ZIP de mundos Java. Nas
configurações, abra o gerenciador de arquivos e a pasta **Worlds** do perfil
escolhido; use **Import** para copiar o mundo. Um ZIP de mundo Java contém
`level.dat` e normalmente uma pasta `region`. A importação usa uma nova pasta,
para não sobrescrever um mundo com o mesmo nome.

Importe cópias e mantenha o original: abrir um mundo antigo no motor atual pode
atualizar seus dados. O motor possui conversores de saves antigos, inclusive do
formato da 1.8, mas isso não significa executar o cliente 1.8.9. Mundos Bedrock,
mods Java, datapacks e recursos específicos de outras versões precisam de
compatibilidade própria. Não há promessa de abrir qualquer mapa sem diferenças.

## Versões

- **26.3 / protocolo 777:** motor atual; mundo plano validado em emulador.
- **1.8–1.8.9 / protocolo 47:** transporte experimental com testes, ainda sem
  integração ao loop do jogo. Não está disponível como versão jogável.
- **Demais versões:** não implementadas como backends selecionáveis.

Os perfis numerados do launcher separam pastas, configurações e mundos. Eles
não trocam a versão do motor. Uma lista de versões no menu só pode representar
suporte real depois que o respectivo backend estiver implementado e testado.
