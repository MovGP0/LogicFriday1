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

        foreach (var dynamicColor in s_materialDynamicColors.AllDynamicColors())
        {
            if (dynamicColor is null)
            {
                continue;
            }

            AddRole(application, ToResourceName(dynamicColor.name), dynamicColor, scheme);
        }

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

    private static string ToResourceName(string dynamicColorName)
    {
        var parts = dynamicColorName.Split('_', StringSplitOptions.RemoveEmptyEntries);
        Span<char> resourceName = stackalloc char[dynamicColorName.Length];
        var length = 0;

        foreach (var part in parts)
        {
            resourceName[length++] = char.ToUpperInvariant(part[0]);

            for (var i = 1; i < part.Length; i++)
            {
                resourceName[length++] = part[i];
            }
        }

        return resourceName[..length].ToString();
    }

    private static Color ToAvaloniaColor(WpfColor color)
    {
        return Color.FromArgb(color.A, color.R, color.G, color.B);
    }
}
