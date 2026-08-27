# Solana Level 1 Token Starter

Учебный starter для итоговых заданий первого уровня курса Superteam KZ. Он показывает современный минимальный каркас токен-программы без привязки к legacy JavaScript SDK.

## Зафиксированный стек

- Anchor CLI и crates: `1.1.2`
- Solana CLI: `3.1.10`
- Rust: `1.89.0`
- тесты программ: Rust + LiteSVM `0.10.0`
- токены: `anchor_spl::token_interface`, совместимый с Token Program и Token-2022
- рекомендуемый клиент для нового TypeScript-кода: `@solana/kit`

`@solana/web3.js` относится к legacy-стеку. TypeScript-клиент Anchor `@anchor-lang/core` по-прежнему зависит от `@solana/web3.js` v1, поэтому в этом starter тесты написаны на Rust и LiteSVM. Для нового клиентского приложения используйте `@solana/kit`, если задание явно не требует другого.

Оригинальный Token Program остается рабочим и широко используется. Для новых токенов в учебных заданиях используйте Token-2022, а program-код пишите через `token_interface`, чтобы сохранить совместимость с обоими Token Program.

## Реализованные инструкции

- создание mint с выбранной token-программой;
- создание associated token account;
- выпуск токенов через `mint_to`;
- перевод через `transfer_checked`;
- сжигание через `anchor_spl::token_interface::burn_checked` с `decimals`, прочитанным из mint;
- проверки положительной суммы, полномочий, mint и token program на уровне Anchor accounts constraints;
- LiteSVM-тесты создания Token-2022 mint и всех обязательных сценариев сжигания.

Escrow намеренно не входит в это задание.

## `burn_tokens`: accounts и инварианты

Инструкция принимает `amount: u64` и проверяет `amount > 0` внутри программы. Нулевая сумма возвращает `TokenStarterError::AmountMustBePositive`; клиентская валидация для безопасности не требуется.

- `authority: Signer` — транзакция обязана содержать подпись владельца сжигаемых токенов.
- `mint: InterfaceAccount<Mint>` — mutable; constraint `mint::token_program = token_program` связывает mint с переданной token-программой.
- `token_account: InterfaceAccount<TokenAccount>` — mutable; constraints `token::mint = mint`, `token::authority = authority` и `token::token_program = token_program` проверяют mint, владельца и программу токена до CPI.
- `token_program: Interface<TokenInterface>` — разрешает только поддерживаемые Token Program и Token-2022; CPI вызывается через этот проверенный аккаунт.

Критичных `UncheckedAccount` нет. После Anchor constraints инструкция берет `mint.decimals` из on-chain состояния и вызывает CPI `burn_checked`; клиент не может подменить decimals. Ошибка constraints или Token Program откатывает всю транзакцию, поэтому mint supply и token balance остаются неизменными.

## Быстрый старт

1. Установите Anchor CLI `1.1.2`, Solana CLI `3.1.10` и Rust `1.89.0`. Версии Anchor и Solana также закреплены в `Anchor.toml`, Rust — в `rust-toolchain.toml`, а crates — точными версиями в `Cargo.toml`/`Cargo.lock`.
2. Проверьте окружение:

   ```console
   anchor --version
   solana --version
   rustc --version
   ```

3. Соберите SBF-программу, затем запустите Rust/LiteSVM-тесты:

   ```console
   anchor build
   cargo test --locked
   ```

   `anchor build` создает `target/deploy/solana_level_1_token_starter.so`, который тесты загружают в LiteSVM. Локальный validator и legacy JavaScript-клиент не нужны.

4. Дополнительные проверки качества:

   ```console
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets --locked -- -D warnings
   ```

## Покрытие `burn_tokens`

Тесты проверяют:

- успешное сжигание и одинаковое уменьшение token account balance и mint supply;
- ожидаемый custom error для нулевой суммы;
- отказ для неверного authority, другого mint, несоответствующей token program и недостаточного баланса;
- байтовую неизменность mint и token account после каждого отказа.

## Правила сдачи

- сдавайте публичную ссылку на GitHub-репозиторий и указывайте ветку или commit SHA;
- добавьте в README команды сборки и тестирования, ожидаемый результат и краткое описание архитектуры;
- не добавляйте в репозиторий private keys, seed phrases, `.env` с секретами или файлы keypair;
- не используйте `@solana/web3.js` в новом клиентском коде;
- для переводов токенов используйте `transfer_checked`, а не unchecked transfer;
- не подменяйте проверки полномочий только клиентской логикой: все критичные инварианты должны проверяться программой.

## Что считается современным решением

Современность здесь определяется не только номером версии. Решение должно использовать строгие account constraints, проверяемые state transitions, Token-2022 для нового токена, `token_interface` для совместимости, `transfer_checked` для переводов и воспроизводимые LiteSVM-тесты. Если официальные стабильные рекомендации Solana или Anchor изменятся, студент должен зафиксировать выбранные версии и объяснить отклонение в README.
