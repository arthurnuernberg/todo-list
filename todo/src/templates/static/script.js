async function saveTitle() {
    const titleElement = document.getElementById('editable-title');
    const newTitle = titleElement.textContent.trim();

    if (newTitle === '') {
        alert('Der Titel darf nicht leer sein.');
        titleElement.textContent = '{{ title }}';
        return;
    }
    const response = await fetch('/update_list_title', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({title: newTitle}),
    });
    if (!response.ok) {
        alert('Fehler beim Speichern der Überschrift.');
    }
}

async function addTag(todoId, tagName) {
    try {
        const response = await fetch('/add_tag', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({todo_id: todoId, tag_name: tagName}),
        });

        if (!response.ok) {
            console.error('Fehler beim Aktualisieren der todo:', response.statusText);
        } else {
            window.location.reload();
        }
    } catch (error) {
        console.error('Fehler beim Senden der Update-Anfrage:', error);
    }
}

async function updateTodoName(todoId, newName) {
    const nameElement = document.getElementById(`title-${todoId}`);
    if (newName === '') {
        alert('Der Titel darf nicht leer sein.');
        nameElement.textContent = nameElement.dataset.originalValue;
        return;
    }

    try {
        const response = await fetch('/update_todo_name', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                todo_id: todoId,
                todo_name: newName,
            }),
        });

        if (!response.ok) {
            console.error('Fehler beim Aktualisieren des Namens des To-dos:', response.statusText);
        }
    } catch (error) {
        console.error('Fehler beim Senden der Update-Anfrage:', error);
    }
}

async function updateTodoDescription(todoId, newDescription) {
    try {
        const response = await fetch('/update_todo_description', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                todo_id: todoId,
                todo_description: newDescription,
            }),
        });

        if (!response.ok) {
            console.error('Fehler beim Aktualisieren der Beschreibung des To-dos:', response.statusText);
        }
    } catch (error) {
        console.error('Fehler beim Senden der Update-Anfrage:', error);
    }
}

async function removeTag(tagId, todoId) {
    try {
        const response = await fetch('/remove_tag', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({todo_id: todoId, tag_id: tagId}),
        });

        if (!response.ok) {
            console.error('Fehler beim Löschen des Tags:', response.statusText);
        }
    } catch (error) {
        console.error('Fehler beim Senden der Anfrage:', error);
    }
    window.location.reload()
}

function updateTagsInput() {
    const select = document.getElementById('tag-filter');
    const selectedValues = Array.from(select.selectedOptions)
        .map(option => option.value)
        .filter(value => value.trim() !== '');
    document.getElementById('tags').value = selectedValues.join(',');
}

async function addTodo(name) {
    try {
        const response = await fetch('/add_todo', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                todo_title: name,
            }),
        })

        if (!response.ok) {
            console.error('Fehler beim Erstellen des To-dos:', response.statusText);
        }
        {
            window.location.reload();
        }
    } catch (error) {
        console.error('Fehler beim Senden der Anfrage:', error);
    }
}

async function tickTodo(todoId) {
    try {
        const response = await fetch('/tick_todo', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({todo_id: todoId}),
        });

        if (!response.ok) {
            console.error('Fehler beim Umschalten des To-dos:', response.statusText);
        }
    } catch (error) {
        console.error('Fehler beim Senden der Anfrage:', error);
    }
}

async function deleteTodo(todoId) {
    try {
        const response = await fetch('/delete_todo', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({todo_id: todoId}),
        });

        if (!response.ok) {
            console.error('Fehler beim Löschen des To-dos:', response.statusText);
        }
        window.location.reload();
    } catch (error) {
        console.error('Fehler beim Senden der Anfrage:', error);
    }
}

async function renameTag(tagId, tagName) {
    try {
        const response = await fetch('/rename_tag', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({tag_id: tagId, tag_name: tagName}),
        });

        if (!response.ok) {
            console.error('Fehler beim Umbenennen des Tags:', response.statusText);
        }
    } catch (error) {
        console.error('Fehler beim Senden der Anfrage:', error);
    }
}

async function addList(newTitle, lists) {

    // Prüfen, ob etwas eingegeben wurde
    if (!newTitle) {
        errorDiv.textContent = "Bitte geben Sie einen Titel ein.";
        return;
    }

    // Prüfen, ob der Titel bereits existiert
    if (lists.includes(newTitle)) {
        errorDiv.textContent = "Diese Liste existiert bereits.";
        return;
    }

    try {
        // Sende den neuen Listentitel als JSON an das Backend (Endpoint: /create_list)
        const response = await fetch('/add_list', {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({list_name: newTitle})
        });

        if (!response.ok) {
            errorDiv.textContent = "Fehler beim Erstellen der Liste.";
        } else {
            // Bei Erfolg: Seite neu laden, um die neue Liste anzuzeigen
            window.location.reload();
        }
    } catch (error) {
        console.error("Fehler:", error);
        errorDiv.textContent = "Ein unerwarteter Fehler ist aufgetreten.";
    }
}

async function deleteList(listId, lists) {
    const confirmed = confirm(`Möchten Sie die Liste wirklich löschen?`)
    if (!confirmed) return;
    if (lists.length < 2) {
        alert("Es muss mindestens eine Liste geben!")
        return;
    }
    try {
        const response = await fetch('/delete_list', {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({list_id: listId})
        });

        if (!response.ok) {
            errorDiv.textContent = `Fehler beim Löschen der Liste mit Id ${listId}.`;
        }
        window.location.reload();
    } catch (error) {
        console.error("Fehler:", error);
        errorDiv.textContent = "Ein unerwarteter Fehler ist aufgetreten.";
    }
}

async function switchList(listName) {
    if (!listName) {
        const selectedOption = Array.from(document.getElementById('list-switch').selectedOptions)[0];
        if (!selectedOption) return;
        listName = selectedOption.value;
    }
    try {
        const response = await fetch('/switch_list', {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({list_name: listName})
        });
        if (!response.ok) {
            errorDiv.textContent = `Fehler beim Wechseln zur Liste mit Id ${listName}.`;
        }
        window.location.reload();
    } catch (error) {
        // console.error("Fehler:", error);
        errorDiv.textContent = "Ein unerwarteter Fehler ist aufgetreten.";
    }
}

// Persistenz über die URL: Filterparameter werden erhalten über die URL
document.addEventListener('DOMContentLoaded', function () {
    // Hole die aktuellen URL-Parameter
    const params = new URLSearchParams(window.location.search);

    // Finde alle Formularfelder innerhalb des Filter-Forms
    const formElements = document.querySelectorAll('.filter-form [name]');
    formElements.forEach(el => {
        const name = el.getAttribute('name');
        if (!name) return;

        // Für Mehrfach-Auswahlen: lese alle Werte aus den Parametern
        if (el.multiple) {
            // Bei Mehrfach-Selects: Es können mehrere Werte existieren
            const values = params.getAll(name);
            Array.from(el.options).forEach(option => {
                option.selected = values.includes(option.value);
            });
        } else {
            // Für normale Felder: Falls ein Wert existiert, setze ihn
            if (params.has(name)) {
                el.value = params.get(name);
            }
        }
    });

    // Falls du eine Funktion hast, die das versteckte "tags"-Feld aktualisiert, aufrufen:
    if (typeof updateTagsInput === 'function') {
        updateTagsInput();
    }

    // Bei Änderungen im Formular die URL aktualisieren, ohne die Seite neu zu laden:
    const filterForm = document.querySelector('.filter-form');
    filterForm.addEventListener('change', function () {
        // Erstelle ein neues FormData-Objekt und wandelt es in URLSearchParams um
        const formData = new FormData(filterForm);
        // Für Mehrfachfelder: Falls das Formular Daten als array sendet, können mehrere Werte vorhanden sein
        const newParams = new URLSearchParams(formData);
        // Aktualisiere die URL (ohne Reload)
        const newUrl = window.location.pathname + '?' + newParams.toString();
        window.history.replaceState({}, '', newUrl);
    });
});

document.addEventListener('DOMContentLoaded', function () {
    // Enter drücken bei Eingabe eines neuen To-do-Namen
    const newTodoInput = document.getElementById('todo_title');
    newTodoInput.addEventListener('keydown', function (event) {
        if (event.key === 'Enter') {
            event.preventDefault();
            addTodo(newTodoInput.value);
        }
    });

    // Enter drücken bei Eingabe eines neuen Listen-Namen
    const newListInput = document.getElementById('newListTitle');
    newListInput.addEventListener('keydown', function (event) {
        if (event.key === 'Enter') {
            event.preventDefault();
            addList(newListInput.value);
        }
    });

    const textInputFields = document.getElementsByClassName('single-line-text');
    for (const field of textInputFields) {
        field.addEventListener('keydown', function (event) {
            if (event.key === 'Enter') {
                event.preventDefault();
                event.target.blur();
            }
        });
    }

    const tagInputFields = document.querySelectorAll('.tag-input');

    tagInputFields.forEach((inputField) => {
        inputField.addEventListener('keydown', function (event) {
            if (event.key === 'Enter') {
                event.preventDefault();

                // Die zugehörige Button-ID basierend auf der Input-ID
                const buttonId = inputField.id.replace('-add_todo', '-add-tag-button');
                const button = document.getElementById(buttonId);

                if (button) {
                    button.click(); // Button-Klick auslösen
                } else {
                    console.error('Kein Button gefunden für Input:', inputField.id);
                }
            }
        });
    });
});