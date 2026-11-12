# Compilação do Pojav Rust

O workflow **Android APK** roda nos pushes em `main` e por acionamento manual.
Ele compila o código deste repositório; não reempacota um APK baixado de terceiros.
Os APKs são assinados e passam pela verificação de conteúdo do projeto antes do
envio aos artifacts. Recursos originais são baixados no aparelho; somente tabelas
geradas permitidas pelo empacotador ficam no APK público.

Os inputs temporários da preparação ficam em `.local/` no runner e nunca são
enviados ao Git ou aos artifacts. O cache guarda dependências Rust e resultados
de compilação, não contas, mundos, inputs do jogo ou material de assinatura.

O identificador Android é `dev.taldoshawn.pojav`, separado de outros launchers e
do Horizon original. A assinatura utiliza os secrets do repositório:

- `ANDROID_KEYSTORE_BASE64`
- `ANDROID_KEYSTORE_PASSWORD`
- `ANDROID_KEY_ALIAS`

Sem esses secrets, o build ainda gera um APK instalável com certificado temporário
de desenvolvimento. Esse artifact não garante atualização sobre uma instalação
de outra execução. Os APKs destinados a uma release devem ser assinados com a
chave permanente, fora do Git, antes da publicação.

O par chave/senha deve ser preservado fora do Git. O certificado precisa ser o
mesmo para atualizar instalações existentes; o número de versão aumenta em cada
execução do workflow. A chave privada não é incluída no APK ou no artifact.

Build manual, após preparar recursos com `tools/android_prepare.py`:

```sh
python3 tools/android_build.py --abi arm64-v8a --public --profile release \
  --app-id dev.taldoshawn.pojav --sdk "$ANDROID_HOME" --java-home "$JAVA_HOME" \
  --gradle "$(command -v gradle)" --ndk-version 28.2.13676358 \
  --build-tools-version 35.0.0 --dev-build local --dev-version-code 1000
```

Troque `arm64-v8a` por `armeabi-v7a`, `x86` ou `x86_64` quando necessário.
Verifique o arquivo `BUILD.json` e o SHA-256 junto do APK. Compilar e assinar não
substitui testar a instalação, a primeira inicialização, o multiplayer e o
desempenho num aparelho. O módulo de protocolo 47 ainda não oferece gameplay.
