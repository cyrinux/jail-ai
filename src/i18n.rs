use std::collections::HashMap;
use std::sync::OnceLock;

/// Supported locales
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    EnUs,
    Es,
    Fr,
    Dk,
}

impl Locale {
    /// Detect locale from environment variables
    pub fn detect() -> Self {
        // Check LANG environment variable
        if let Ok(lang) = std::env::var("LANG") {
            let lang = lang.to_lowercase();
            if lang.starts_with("es") {
                return Locale::Es;
            } else if lang.starts_with("fr") {
                return Locale::Fr;
            } else if lang.starts_with("dk") {
                return Locale::Dk;
            } else if lang.starts_with("en_us") {
                return Locale::EnUs;
            }
        }
        
        // Check LANGUAGE environment variable as fallback
        if let Ok(lang) = std::env::var("LANGUAGE") {
            let lang = lang.to_lowercase();
            if lang.starts_with("es") {
                return Locale::Es;
            } else if lang.starts_with("fr") {
                return Locale::Fr;
            } else if lang.starts_with("dk") {
                return Locale::Dk;
            } else if lang.starts_with("en_us") {
                return Locale::EnUs;
            }
        }
        
        Locale::En // Default to English
    }
}

/// Translation key
pub enum Msg {
    UpdateAvailable,
    OutdatedLayersDetected,
    OutdatedLayersExplain,
    ContainerImageMismatch,
    Current,
    Expected,
    RecommendationUseUpgrade,
    RebuildOutdatedLayers,
    RecreateContainer,
    EnsureLatestTools,
    DataPreserved,
    WouldYouLikeRebuild,
    CheckingUpdates,
    UserChoseUpgrade,
    UserDeclinedUpgrade,
    ContainerUpToDate,
    CreatingNewJail,
    RecreatingJailUpgrade,
    RecreatingJailDetectedUpdates,
}

type Translations = HashMap<&'static str, &'static str>;

/// Get translations for a specific locale
fn translations(locale: Locale) -> &'static Translations {
    static EN: OnceLock<Translations> = OnceLock::new();
    static ES: OnceLock<Translations> = OnceLock::new();
    static FR: OnceLock<Translations> = OnceLock::new();
    static DK: OnceLock<Translations> = OnceLock::new();
    
    match locale {
        Locale::En | Locale::EnUs => EN.get_or_init(|| {
            let mut m = HashMap::new();
            m.insert("update_available", "\n🔄 Update available for your jail environment!");
            m.insert("outdated_layers_detected", "\n📦 Outdated layers detected:");
            m.insert("outdated_layers_explain", "\nThis typically happens after upgrading the jail-ai binary.\nLayers contain updated tools, dependencies, or security patches.");
            m.insert("container_image_mismatch", "\n🐳 Container image mismatch:");
            m.insert("current", "  Current:  {}");
            m.insert("expected", "  Expected: {}");
            m.insert("recommendation_use_upgrade", "\n💡 Recommendation: Use --upgrade to:");
            m.insert("rebuild_outdated_layers", "  • Rebuild outdated layers with latest definitions");
            m.insert("recreate_container", "  • Recreate container with the correct image");
            m.insert("ensure_latest_tools", "  • Ensure you have the latest tools and security patches");
            m.insert("data_preserved", "\nYour data in /home/agent will be preserved during the rebuild.");
            m.insert("would_you_like_rebuild", "\nWould you like to rebuild now? (y/N): ");
            m.insert("checking_updates", "Checking for updates...");
            m.insert("user_chose_upgrade", "User chose to upgrade");
            m.insert("user_declined_upgrade", "User declined rebuild, continuing with existing container");
            m.insert("container_up_to_date", "Container and layers are up to date");
            m.insert("creating_new_jail", "Creating new jail: {}");
            m.insert("recreating_jail_upgrade", "Recreating jail '{}' due to --upgrade or --layers");
            m.insert("recreating_jail_detected_updates", "Recreating jail '{}' due to detected updates");
            m
        }),
        Locale::Fr => FR.get_or_init(|| {
            let mut m = HashMap::new();
            m.insert("update_available", "\n🔄 Mise à jour disponible pour votre environnement jail !");
            m.insert("outdated_layers_detected", "\n📦 Couches obsolètes détectées :");
            m.insert("outdated_layers_explain", "\nCela se produit généralement après la mise à jour du binaire jail-ai.\nLes couches contiennent des outils, des dépendances ou des correctifs de sécurité mis à jour.");
            m.insert("container_image_mismatch", "\n🐳 Incohérence de l'image du conteneur :");
            m.insert("current", "  Actuelle : {}");
            m.insert("expected", "  Attendue : {}");
            m.insert("recommendation_use_upgrade", "\n💡 Recommandation : Utilisez --upgrade pour :");
            m.insert("rebuild_outdated_layers", "  • Reconstruire les couches obsolètes avec les dernières définitions");
            m.insert("recreate_container", "  • Recréer le conteneur avec l'image correcte");
            m.insert("ensure_latest_tools", "  • Vous assurer d'avoir les derniers outils et correctifs de sécurité");
            m.insert("data_preserved", "\nVos données dans /home/agent seront préservées pendant la reconstruction.");
            m.insert("would_you_like_rebuild", "\nSouhaitez-vous reconstruire maintenant ? (o/N) : ");
            m.insert("checking_updates", "Vérification des mises à jour...");
            m.insert("user_chose_upgrade", "L'utilisateur a choisi de mettre à jour");
            m.insert("user_declined_upgrade", "L'utilisateur a refusé la reconstruction, poursuite avec le conteneur existant");
            m.insert("container_up_to_date", "Le conteneur et les couches sont à jour");
            m.insert("creating_new_jail", "Création d'un nouveau jail : {}");
            m.insert("recreating_jail_upgrade", "Recréation du jail '{}' en raison de --upgrade ou --layers");
            m.insert("recreating_jail_detected_updates", "Recréation du jail '{}' en raison de mises à jour détectées");
            m
        }),
        Locale::Es => ES.get_or_init(|| {
            let mut m = HashMap::new();
            m.insert("update_available", "\n🔄 ¡Actualización disponible para tu entorno jail!");
            m.insert("outdated_layers_detected", "\n📦 Capas desactualizadas detectadas:");
            m.insert("outdated_layers_explain", "\nEsto suele ocurrir después de actualizar el binario jail-ai.\nLas capas contienen herramientas, dependencias o parches de seguridad actualizados.");
            m.insert("container_image_mismatch", "\n🐳 Discrepancia en la imagen del contenedor:");
            m.insert("current", "  Actual:   {}");
            m.insert("expected", "  Esperada: {}");
            m.insert("recommendation_use_upgrade", "\n💡 Recomendación: Use --upgrade para:");
            m.insert("rebuild_outdated_layers", "  • Reconstruir capas desactualizadas con las últimas definiciones");
            m.insert("recreate_container", "  • Recrear contenedor con la imagen correcta");
            m.insert("ensure_latest_tools", "  • Asegurar que tiene las últimas herramientas y parches de seguridad");
            m.insert("data_preserved", "\nSus datos en /home/agent se conservarán durante la reconstrucción.");
            m.insert("would_you_like_rebuild", "\n¿Desea reconstruir ahora? (s/N): ");
            m.insert("checking_updates", "Comprobando actualizaciones...");
            m.insert("user_chose_upgrade", "El usuario eligió actualizar");
            m.insert("user_declined_upgrade", "El usuario rechazó la reconstrucción, continuando con el contenedor existente");
            m.insert("container_up_to_date", "El contenedor y las capas están actualizados");
            m.insert("creating_new_jail", "Creando nuevo jail: {}");
            m.insert("recreating_jail_upgrade", "Recreando jail '{}' debido a --upgrade o --layers");
            m.insert("recreating_jail_detected_updates", "Recreando jail '{}' debido a actualizaciones detectadas");
            m
        }),
        Locale::Dk => DK.get_or_init(|| {
            let mut m = HashMap::new();
            m.insert("update_available", "\n🔄 Opdatering tilgængelig for dit jail-miljø!");
            m.insert("outdated_layers_detected", "\n📦 Forældede lag opdaget:");
            m.insert("outdated_layers_explain", "\nDette sker typisk efter opgradering af jail-ai-binaren.\nLag indeholder opdaterede værktøjer, afhængigheder eller sikkerhedsrettelser.");
            m.insert("container_image_mismatch", "\n🐳 Uoverensstemmelse i container-image:");
            m.insert("current", "  Nuværende: {}");
            m.insert("expected", "  Forventet:  {}");
            m.insert("recommendation_use_upgrade", "\n💡 Anbefaling: Brug --upgrade til at:");
            m.insert("rebuild_outdated_layers", "  • Genopbygge forældede lag med de seneste definitioner");
            m.insert("recreate_container", "  • Genoprette containeren med det korrekte image");
            m.insert("ensure_latest_tools", "  • Sikre, at du har de nyeste værktøjer og sikkerhedsrettelser");
            m.insert("data_preserved", "\nDine data i /home/agent vil blive bevaret under genopbygningen.");
            m.insert("would_you_like_rebuild", "\nVil du genopbygge nu? (y/N): ");
            m.insert("checking_updates", "Søger efter opdateringer...");
            m.insert("user_chose_upgrade", "Brugeren valgte at opgradere");
            m.insert("user_declined_upgrade", "Brugeren afviste genopbygning, fortsætter med eksisterende container");
            m.insert("container_up_to_date", "Container og lag er opdaterede");
            m.insert("creating_new_jail", "Opretter nyt jail: {}");
            m.insert("recreating_jail_upgrade", "Genopretter jail '{}' på grund af --upgrade eller --layers");
            m.insert("recreating_jail_detected_updates", "Genopretter jail '{}' på grund af opdagede opdateringer");
            m
        }),
    }
}

/// Global locale setting
static LOCALE: OnceLock<Locale> = OnceLock::new();

/// Initialize the locale
pub fn init() {
    LOCALE.get_or_init(Locale::detect);
}

/// Get the current locale
pub fn locale() -> Locale {
    *LOCALE.get_or_init(Locale::detect)
}

/// Get a translated message
pub fn t(key: &str) -> String {
    let locale = locale();
    match translations(locale).get(key) {
        Some(translation) => translation.to_string(),
        None => {
            // For unknown keys, return a fallback
            match key {
                "update_available" => "\n🔄 Update available for your jail environment!".to_string(),
                "outdated_layers_detected" => "\n📦 Outdated layers detected:".to_string(),
                "outdated_layers_explain" => "\nThis typically happens after upgrading the jail-ai binary.\nLayers contain updated tools, dependencies, or security patches.".to_string(),
                "container_image_mismatch" => "\n🐳 Container image mismatch:".to_string(),
                "current" => "  Current:  {}".to_string(),
                "expected" => "  Expected: {}".to_string(),
                "recommendation_use_upgrade" => "\n💡 Recommendation: Use --upgrade to:".to_string(),
                "rebuild_outdated_layers" => "  • Rebuild outdated layers with latest definitions".to_string(),
                "recreate_container" => "  • Recreate container with the correct image".to_string(),
                "ensure_latest_tools" => "  • Ensure you have the latest tools and security patches".to_string(),
                "data_preserved" => "\nYour data in /home/agent will be preserved during the rebuild.".to_string(),
                "would_you_like_rebuild" => "\nWould you like to rebuild now? (y/N): ".to_string(),
                "checking_updates" => "Checking for updates...".to_string(),
                "user_chose_upgrade" => "User chose to upgrade".to_string(),
                "user_declined_upgrade" => "User declined rebuild, continuing with existing container".to_string(),
                "container_up_to_date" => "Container and layers are up to date".to_string(),
                "creating_new_jail" => "Creating new jail: {}".to_string(),
                "recreating_jail_upgrade" => "Recreating jail '{}' due to --upgrade or --layers".to_string(),
                "recreating_jail_detected_updates" => "Recreating jail '{}' due to detected updates".to_string(),
                _ => key.to_string(), // Fallback to key itself
            }
        }
    }
}

/// Format a translated message with arguments
pub fn tf(key: &str, args: &[&dyn std::fmt::Display]) -> String {
    let template = t(key);
    if args.is_empty() {
        return template.to_string();
    }
    
    // Simple string formatting
    let mut result = template.to_string();
    for (i, arg) in args.iter().enumerate() {
        let placeholder = format!("{{{}}}", i);
        result = result.replace(&placeholder, &arg.to_string());
    }
    result
}

/// Helper function for single argument formatting
pub fn tf1(key: &str, arg: &dyn std::fmt::Display) -> String {
    t(key).replace("{}", &arg.to_string())
}
