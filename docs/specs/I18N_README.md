# Internationalization (i18n) Support

jail-ai now supports multiple languages for user-facing messages!

## Supported Languages

- **English (en)** - Default
- **Spanish (es)** 
- **French (fr)**

## How It Works

The locale is automatically detected from your system environment variables:
- `LANG` 
- `LANGUAGE` (fallback)

### Examples

```bash
# Use Spanish
export LANG=es_ES.UTF-8
jail-ai claude

# Use French  
export LANG=fr_FR.UTF-8
jail-ai claude

# Use English (default)
export LANG=en_US.UTF-8
jail-ai claude
```

## Supported Responses

The prompt accepts locale-specific affirmative responses:
- **English**: `y`, `yes`
- **Spanish**: `s`, `si`
- **French**: `o`, `oui`

## Example Output

### English
```
🔄 Update available for your jail environment!

📦 Outdated layers detected:
  • base
  • rust

💡 Recommendation: Use --upgrade to:
  • Rebuild outdated layers with latest definitions
  • Ensure you have the latest tools and security patches

Your data in /home/agent will be preserved during the rebuild.

Would you like to rebuild now? (y/N):
```

### Spanish (Español)
```
🔄 ¡Actualización disponible para tu entorno jail!

📦 Capas desactualizadas detectadas:
  • base
  • rust

💡 Recomendación: Use --upgrade para:
  • Reconstruir capas desactualizadas con las últimas definiciones
  • Asegurar que tiene las últimas herramientas y parches de seguridad

Sus datos en /home/agent se conservarán durante la reconstrucción.

¿Desea reconstruir ahora? (s/N):
```

### French (Français)
```
🔄 Mise à jour disponible pour votre environnement jail !

📦 Couches obsolètes détectées :
  • base
  • rust

💡 Recommandation : Utilisez --upgrade pour :
  • Reconstruire les couches obsolètes avec les dernières définitions
  • Vous assurer d'avoir les derniers outils et correctifs de sécurité

Vos données dans /home/agent seront préservées pendant la reconstruction.

Souhaitez-vous reconstruire maintenant ? (o/N) :
```

## Implementation Details

- Simple HashMap-based translation system in `src/i18n.rs`
- No external dependencies required
- Uses `OnceLock` for thread-safe lazy initialization
- Automatic locale detection on startup
- Fallback to English if locale not supported

## Adding More Languages

To add support for additional languages:

1. Edit `src/i18n.rs`
2. Add a new `Locale` variant
3. Add detection logic in `Locale::detect()`
4. Create a new translation map in the `translations()` function
5. Update the prompt response logic in `prompt_upgrade()` if needed

Example for German:

```rust
// In Locale enum
Locale::De,

// In detect()
if lang.starts_with("de") {
    return Locale::De;
}

// In translations()
Locale::De => DE.get_or_init(|| {
    let mut m = HashMap::new();
    m.insert("update_available", "\n🔄 Aktualisierung verfügbar!");
    // ... more translations
    m
}),
```
