/* 00423a22 FUN_00423a22 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

void __fastcall FUN_00423a22(int param_1)

{
  int iVar1;
  uint unaff_retaddr;
  uint local_194 [65];
  uint local_90;
  undefined1 local_8c [4];
  uint local_88;
  uint local_84;
  uint local_80;
  uint local_7c;
  uint local_78;
  uint local_74;
  uint local_70;
  uint local_6c;
  uint local_68;
  uint local_64;
  uint local_60;
  uint local_5c;
  uint local_58;
  uint local_54;
  uint local_50;
  int local_4c;
  int local_48;
  int local_44;
  int local_40;
  int local_3c;
  int local_38;
  int local_34;
  int local_30;
  int local_2c;
  int local_28;
  int local_24;
  int local_20;
  int local_1c;
  int local_18;
  int local_14;
  int local_10;
  int local_8;
  
  local_90 = DAT_00451a00 ^ unaff_retaddr;
  _memset(local_8c,0,0x80);
  for (local_8 = *(int *)(param_1 + 0x1654); local_8 < *(int *)(param_1 + 0x1658);
      local_8 = local_8 + 1) {
    if (*(int *)(*(int *)(param_1 + 0x3a4) + 0x48 + local_8 * 0xfc) == 0) {
      switch(*(undefined4 *)(local_8 * 0xfc + *(int *)(param_1 + 0x3a4))) {
      case 0:
        local_88 = local_88 + 1;
        break;
      case 1:
        iVar1 = *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_8 * 0xfc);
        if (iVar1 == 2) {
          local_84 = local_84 + 1;
        }
        else if (iVar1 == 3) {
          local_80 = local_80 + 1;
        }
        else if (iVar1 == 4) {
          local_7c = local_7c + 1;
        }
        break;
      case 2:
        iVar1 = *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_8 * 0xfc);
        if (iVar1 == 2) {
          local_78 = local_78 + 1;
        }
        else if (iVar1 == 3) {
          local_74 = local_74 + 1;
        }
        else if (iVar1 == 4) {
          local_70 = local_70 + 1;
        }
        break;
      case 3:
        local_54 = local_54 + 1;
        break;
      default:
        return;
      case 5:
        local_50 = local_50 + 1;
        break;
      case 6:
        iVar1 = *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_8 * 0xfc);
        if (iVar1 == 2) {
          local_6c = local_6c + 1;
        }
        else if (iVar1 == 3) {
          local_68 = local_68 + 1;
        }
        else if (iVar1 == 4) {
          local_64 = local_64 + 1;
        }
        break;
      case 7:
        iVar1 = *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_8 * 0xfc);
        if (iVar1 == 2) {
          local_60 = local_60 + 1;
        }
        else if (iVar1 == 3) {
          local_5c = local_5c + 1;
        }
        else if (iVar1 == 4) {
          local_58 = local_58 + 1;
        }
        break;
      case 8:
        break;
      case 10:
        break;
      case 0xb:
      }
    }
  }
  if (local_88 != 0) {
    for (; (uint)(local_48 * 6) < local_88; local_48 = local_48 + 1) {
    }
    local_4c = local_4c + local_48;
  }
  if (local_84 != 0) {
    for (; (uint)(local_44 << 2) < local_84; local_44 = local_44 + 1) {
    }
    local_4c = local_4c + local_44;
  }
  if (local_80 != 0) {
    for (; (uint)(local_40 * 3) < local_80; local_40 = local_40 + 1) {
    }
    local_4c = local_4c + local_40;
  }
  if (local_7c != 0) {
    for (; (uint)(local_3c << 1) < local_7c; local_3c = local_3c + 1) {
    }
    local_4c = local_4c + local_3c;
  }
  if (local_78 != 0) {
    for (; (uint)(local_38 << 2) < local_78; local_38 = local_38 + 1) {
    }
    local_4c = local_4c + local_38;
  }
  if (local_74 != 0) {
    for (; (uint)(local_34 * 3) < local_74; local_34 = local_34 + 1) {
    }
    local_4c = local_4c + local_34;
  }
  if (local_70 != 0) {
    for (; (uint)(local_30 << 1) < local_70; local_30 = local_30 + 1) {
    }
    local_4c = local_4c + local_30;
  }
  if (local_6c != 0) {
    for (; (uint)(local_2c << 2) < local_6c; local_2c = local_2c + 1) {
    }
    local_4c = local_4c + local_2c;
  }
  if (local_68 != 0) {
    for (; (uint)(local_28 * 3) < local_68; local_28 = local_28 + 1) {
    }
    local_4c = local_4c + local_28;
  }
  if (local_64 != 0) {
    for (; (uint)(local_24 << 1) < local_64; local_24 = local_24 + 1) {
    }
    local_4c = local_4c + local_24;
  }
  if (local_60 != 0) {
    for (; (uint)(local_20 << 2) < local_60; local_20 = local_20 + 1) {
    }
    local_4c = local_4c + local_20;
  }
  if (local_5c != 0) {
    for (; (uint)(local_1c * 3) < local_5c; local_1c = local_1c + 1) {
    }
    local_4c = local_4c + local_1c;
  }
  if (local_58 != 0) {
    for (; (uint)(local_18 << 1) < local_58; local_18 = local_18 + 1) {
    }
    local_4c = local_4c + local_18;
  }
  if (local_54 != 0) {
    for (; (uint)(local_14 << 2) < local_54; local_14 = local_14 + 1) {
    }
    local_4c = local_4c + local_14;
  }
  if (local_50 != 0) {
    for (; (uint)(local_10 << 2) < local_50; local_10 = local_10 + 1) {
    }
    local_4c = local_4c + local_10;
  }
  if (local_4c != 0) {
    FUN_0043ebd0(*(uint **)(param_1 + 0x274),(uint *)"IC\tQty\r\n");
    if (local_48 != 0) {
      FUN_0043ed39((char *)local_194,(byte *)"Hex Inverter\t%d\r\n");
      FUN_0043ebe0(*(uint **)(param_1 + 0x274),local_194);
    }
    if (local_44 != 0) {
      FUN_0043ed39((char *)local_194,(byte *)"Quad 2-Input NAND\t%d\r\n");
      FUN_0043ebe0(*(uint **)(param_1 + 0x274),local_194);
    }
    if (local_40 != 0) {
      FUN_0043ed39((char *)local_194,(byte *)"Triple 3-Input NAND\t%d\r\n");
      FUN_0043ebe0(*(uint **)(param_1 + 0x274),local_194);
    }
    if (local_3c != 0) {
      FUN_0043ed39((char *)local_194,(byte *)"Dual 4-Input NAND\t%d\r\n");
      FUN_0043ebe0(*(uint **)(param_1 + 0x274),local_194);
    }
    if (local_38 != 0) {
      FUN_0043ed39((char *)local_194,(byte *)"Quad 2-Input NOR\t%d\r\n");
      FUN_0043ebe0(*(uint **)(param_1 + 0x274),local_194);
    }
    if (local_34 != 0) {
      FUN_0043ed39((char *)local_194,(byte *)"Triple 3-Input NOR\t%d\r\n");
      FUN_0043ebe0(*(uint **)(param_1 + 0x274),local_194);
    }
    if (local_30 != 0) {
      FUN_0043ed39((char *)local_194,(byte *)"Dual 4-Input NOR\t%d\r\n");
      FUN_0043ebe0(*(uint **)(param_1 + 0x274),local_194);
    }
    if (local_2c != 0) {
      FUN_0043ed39((char *)local_194,(byte *)"Quad 2-Input AND\t%d\r\n");
      FUN_0043ebe0(*(uint **)(param_1 + 0x274),local_194);
    }
    if (local_28 != 0) {
      FUN_0043ed39((char *)local_194,(byte *)"Triple 3-Input AND\t%d\r\n");
      FUN_0043ebe0(*(uint **)(param_1 + 0x274),local_194);
    }
    if (local_24 != 0) {
      FUN_0043ed39((char *)local_194,(byte *)"Dual 4-Input AND\t%d\r\n");
      FUN_0043ebe0(*(uint **)(param_1 + 0x274),local_194);
    }
    if (local_20 != 0) {
      FUN_0043ed39((char *)local_194,(byte *)"Quad 2-Input OR\t%d\r\n");
      FUN_0043ebe0(*(uint **)(param_1 + 0x274),local_194);
    }
    if (local_1c != 0) {
      FUN_0043ed39((char *)local_194,(byte *)"Triple 3-Input OR\t%d\r\n");
      FUN_0043ebe0(*(uint **)(param_1 + 0x274),local_194);
    }
    if (local_18 != 0) {
      FUN_0043ed39((char *)local_194,(byte *)"Dual 4-Input OR\t%d\r\n");
      FUN_0043ebe0(*(uint **)(param_1 + 0x274),local_194);
    }
    if (local_14 != 0) {
      FUN_0043ed39((char *)local_194,(byte *)"Quad 2-Input EXOR\t%d\r\n");
      FUN_0043ebe0(*(uint **)(param_1 + 0x274),local_194);
    }
    if (local_10 != 0) {
      FUN_0043ed39((char *)local_194,(byte *)"Quad 2-Input MUX\t%d\r\n");
      FUN_0043ebe0(*(uint **)(param_1 + 0x274),local_194);
    }
    FUN_0043ed39((char *)local_194,(byte *)"TOTAL PACKAGES\t%d\r\n");
    FUN_0043ebe0(*(uint **)(param_1 + 0x274),local_194);
  }
  return;
}
