# Responsibility Map

Поставляемый с erlu WASM-плагин для распределения ответственности и экспорта
матрицы RACI. Доступен в меню **Plugins** в desktop и web; его действия также
доступны через MCP и web API.

## Работа в интерфейсе

1. **Responsibility Map — Add role…**: добавьте организационные роли, например
   «Оператор», «Руководитель», «Бухгалтерия».
2. Выделите один или несколько функциональных блоков. **Assign selected functions…**:
   введите название зарегистрированной роли и выберите категорию:
   - **R — Responsible**: выполняет работу;
   - **A — Accountable**: отвечает за результат;
   - **C — Consulted**: участвует в консультациях;
   - **I — Informed**: получает информацию.
3. По умолчанию назначение добавляется к существующим. **Replace other roles in
   this category** заменяет только выбранную категорию на выделенных функциях.
   Одна роль может одновременно иметь R и A.
4. **Check responsibility…** проверяет всю модель, активную диаграмму либо выделение.
5. **Export RACI matrix…** формирует Markdown, CSV или JSON. В окне отчёта доступны
   **Copy report** и **Save report…**. В web сохранение скачивает файл.

Регистр названий не важен; пробелы по краям удаляются. Переименование сохраняет
назначения. Удаление роли удаляет и её назначения во всей модели. **Remove assignment
from selection…** удаляет только одну роль из одной категории, а **Clear assignments
for selection** очищает все назначения выделенных функций. Все изменения отменяются
через Undo/Redo.

Роли задаются явно для каждой функции. Назначения родителя не наследуются дочерними
функциями. Стрелки механизмов IDEF0 не преобразуются в роли автоматически:
механизмом может быть оборудование или программная система.

## Политика проверки

Это политика RACI данного плагина, не обязательные требования ISO/IEC/IEEE 31320-1.
Она требует хотя бы одного R и ровно одного A у каждой проверяемой функции.
C и I необязательны. Несколько R допустимы. Проверки не препятствуют сохранению
или редактированию модели.

Диагностики `missing_responsible`, `missing_accountable`, `multiple_accountable`
и `unknown_role` имеют уровень error; `orphan_assignment` — warning. Их `source`
равен `responsibility_map`. После удаления функции её записи остаются в расширении
для прозрачного обнаружения устаревших ссылок; **Prune deleted functions** удаляет
такие записи одной отменяемой транзакцией. Экспорт содержит только существующие
функции.

## API и хранение

Идентификатор плагина: `responsibility_map`; версия схемы данных: 1.
Данные находятся только в `extensions.responsibility_map` и сохраняются вместе с
`.idef0`. Это дополнительное описание ответственности: семантику IDEF0 оно не меняет
и обязательной capability не вводит. При отсутствии плагина erlu сохраняет данные
расширения без потерь. Неизвестная версия данных отклоняется без перезаписи.

| Действие MCP/web | Аргументы |
| --- | --- |
| `plugin_responsibility_map_role_add` | `name` |
| `plugin_responsibility_map_role_rename` | `name`, `new_name` |
| `plugin_responsibility_map_role_remove` | `name` |
| `plugin_responsibility_map_assignment_set` | `role`, `responsibility`, необязательный `replace: false` |
| `plugin_responsibility_map_assignment_remove` | `role`, `responsibility` |
| `plugin_responsibility_map_assignment_clear` | — |
| `plugin_responsibility_map_data_prune` | — |
| `plugin_responsibility_map_matrix_export` | `scope: document/diagram/selection`, `format: markdown/csv/json` |
| `plugin_responsibility_map_validate` | `scope: document/diagram/selection` |

Назначения используют текущее выделение; выбирайте его действием `selection_set`.
Редактирующие вызовы следует выполнять с актуальным `expected_revision`.
Внутренние ID ролей стабильны при переименовании. Назначения ссылаются на стабильные
ID блоков, а не на изменяемые номера A1/A11. Экспорт и проверка не предлагают команд
редактирования и не меняют ревизию документа.

Экспорт возвращает `result.text`, `mime`, `filename`, `row_count`. JSON-матрица
содержит массив `roles` (`id`, `name`) и `rows` (`block`, `diagram`, `node`,
`function`, списки ID ролей `R`, `A`, `C`, `I`). Столбцы Markdown/CSV — номер функции,
название и роли. Формулы в текстовых CSV-ячейках обезвреживаются префиксом апострофа;
кавычки и переносы строк сохраняются по правилам CSV. Markdown экранирует содержимое
ячеек. Матрица ограничена 200 000 ячеек «функция × роль», реестр — 256 ролями;
для больших моделей выбирайте диаграмму или выделение.

## Сборка и проверки

```sh
npm --prefix web ci
npm --prefix web run plugins
cargo test --manifest-path plugins/responsibility_map/Cargo.toml --locked
cargo test --workspace --locked
trunk build --release --locked
```

Плагин использует тот же WIT-контракт, что и colors, без доступа к сети и файловой
системе. Native исполняет компонент в Wasmtime, браузер — в отдельном Worker.
