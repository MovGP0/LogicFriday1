using Avalonia;
using Avalonia.Media;
using Avalonia.Styling;
using MaterialColorUtilities;
using WpfColor = System.Windows.Media.Color;

namespace LogicFriday1.Styling;

public static class MaterialColorSchemeInstaller
{
    private static readonly MaterialDynamicColors s_materialDynamicColors = new();

    public static void Install(Application application)
    {
        application.RequestedThemeVariant = ThemeVariant.Light;

        var sourceColor = WpfColor.FromRgb(0x39, 0x49, 0xAB);
        var scheme = DynamicSchemeFactory.Create(
            sourceColor,
            Variant.TonalSpot,
            isDark: false,
            contrastLevel: 0,
            Platform.Phone,
            SpecVersion.Spec2025,
            primary: sourceColor,
            secondary: null,
            tertiary: null,
            neutral: null,
            neutralVariant: null,
            error: null);

        AddRole(application, "Primary", s_materialDynamicColors.Primary, scheme);
        AddRole(application, "OnPrimary", s_materialDynamicColors.OnPrimary, scheme);
        AddRole(application, "PrimaryContainer", s_materialDynamicColors.PrimaryContainer, scheme);
        AddRole(application, "OnPrimaryContainer", s_materialDynamicColors.OnPrimaryContainer, scheme);
        AddRole(application, "SecondaryContainer", s_materialDynamicColors.SecondaryContainer, scheme);
        AddRole(application, "OnSecondaryContainer", s_materialDynamicColors.OnSecondaryContainer, scheme);
        AddRole(application, "Surface", s_materialDynamicColors.Surface, scheme);
        AddRole(application, "SurfaceContainerLowest", s_materialDynamicColors.SurfaceContainerLowest, scheme);
        AddRole(application, "SurfaceContainerLow", s_materialDynamicColors.SurfaceContainerLow, scheme);
        AddRole(application, "SurfaceContainer", s_materialDynamicColors.SurfaceContainer, scheme);
        AddRole(application, "SurfaceContainerHigh", s_materialDynamicColors.SurfaceContainerHigh, scheme);
        AddRole(application, "SurfaceContainerHighest", s_materialDynamicColors.SurfaceContainerHighest, scheme);
        AddRole(application, "OnSurface", s_materialDynamicColors.OnSurface, scheme);
        AddRole(application, "OnSurfaceVariant", s_materialDynamicColors.OnSurfaceVariant, scheme);
        AddRole(application, "Outline", s_materialDynamicColors.Outline, scheme);
        AddRole(application, "OutlineVariant", s_materialDynamicColors.OutlineVariant, scheme);
        AddRole(application, "Error", s_materialDynamicColors.Error, scheme);
        AddRole(application, "OnError", s_materialDynamicColors.OnError, scheme);

        application.Resources["SystemAccentColor"] = ToAvaloniaColor(s_materialDynamicColors.Primary.GetColor(scheme));
        application.Resources["SystemAccentColorBrush"] = application.Resources["LogicFriday.Brush.Primary"];
        application.Resources["SystemControlForegroundBaseLowBrush"] = application.Resources["LogicFriday.Brush.OutlineVariant"];
    }

    private static void AddRole(Application application, string name, DynamicColor dynamicColor, DynamicScheme scheme)
    {
        var color = ToAvaloniaColor(dynamicColor.GetColor(scheme));

        application.Resources[$"LogicFriday.Color.{name}"] = color;
        application.Resources[$"LogicFriday.Brush.{name}"] = new SolidColorBrush(color);
    }

    private static Color ToAvaloniaColor(WpfColor color)
    {
        return Color.FromArgb(color.A, color.R, color.G, color.B);
    }
}
