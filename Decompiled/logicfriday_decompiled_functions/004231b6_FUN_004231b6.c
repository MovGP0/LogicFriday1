/* 004231b6 FUN_004231b6 */

undefined4 __fastcall FUN_004231b6(int param_1)

{
  bool bVar1;
  bool bVar2;
  int local_68;
  int local_64;
  int local_60;
  int local_5c;
  int local_58;
  int local_54;
  int local_50;
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
  int local_1c;
  int local_14;
  int local_10;
  int local_c;
  int local_8;
  
  *(undefined4 *)(param_1 + 0x2308) = 0;
  bVar1 = *(int *)(param_1 + 0x3cc + *(int *)(param_1 + 0x2304) * 0x118) == 0;
  bVar2 = *(int *)(param_1 + 0x3b4) == 0x403;
  if ((bVar2) || (bVar1)) {
    if (*(int *)(param_1 + 0x3cc + *(int *)(param_1 + 0x22ec) * 0x118) == 0) {
      if (*(int *)(param_1 + 0x3cc + *(int *)(param_1 + 0x22f8) * 0x118) == 0) {
        if (*(int *)(param_1 + 0x3cc + *(int *)(param_1 + 0x22f0) * 0x118) == 0) {
          if (*(int *)(param_1 + 0x3cc + *(int *)(param_1 + 0x22fc) * 0x118) == 0) {
            if (*(int *)(param_1 + 0x3cc + *(int *)(param_1 + 0x22f4) * 0x118) == 0) {
              if (*(int *)(param_1 + 0x3cc + *(int *)(param_1 + 0x2300) * 0x118) == 0) {
                return 0x230003;
              }
              local_14 = 2;
              local_1c = 4;
            }
            else {
              local_14 = 1;
              local_1c = 4;
            }
          }
          else {
            local_14 = 2;
            local_1c = 3;
          }
        }
        else {
          local_14 = 1;
          local_1c = 3;
        }
      }
      else {
        local_14 = 2;
        local_1c = 2;
      }
    }
    else {
      local_14 = 1;
      local_1c = 2;
    }
    if (bVar2) {
      local_34 = 0;
      local_44 = 0;
      local_54 = 0;
      local_2c = 0;
      local_5c = 0;
      local_50 = 0;
      local_28 = 0;
      for (local_c = *(int *)(param_1 + 0x1654); local_c < *(int *)(param_1 + 0x1658);
          local_c = local_c + 1) {
        if (*(int *)(*(int *)(param_1 + 0x3a4) + 0x48 + local_c * 0xfc) == 0) {
          if (*(int *)(local_c * 0xfc + *(int *)(param_1 + 0x3a4)) == 1) {
            if (*(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_c * 0xfc) == 2) {
              local_34 = local_34 + 1;
            }
            else if (*(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_c * 0xfc) == 3) {
              local_44 = local_44 + 1;
            }
            else if (*(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_c * 0xfc) == 4) {
              local_54 = local_54 + 1;
            }
          }
          else if (*(int *)(local_c * 0xfc + *(int *)(param_1 + 0x3a4)) == 2) {
            if (*(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_c * 0xfc) == 2) {
              local_2c = local_2c + 1;
            }
            else if (*(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_c * 0xfc) == 3) {
              local_5c = local_5c + 1;
            }
            else if (*(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_c * 0xfc) == 4) {
              local_50 = local_50 + 1;
            }
          }
          else if (*(int *)(local_c * 0xfc + *(int *)(param_1 + 0x3a4)) == 0) {
            local_28 = local_28 + 1;
          }
        }
      }
      if (local_28 != 0) {
        local_48 = FUN_004242ed(local_34,4);
        local_68 = FUN_004242ed(local_44,3);
        local_40 = FUN_004242ed(local_54,2);
        local_64 = FUN_004242ed(local_2c,4);
        local_60 = FUN_004242ed(local_5c,3);
        local_3c = FUN_004242ed(local_50,2);
        local_58 = local_48 + local_68 + local_40 + local_64 + local_60 + local_3c;
        if (bVar1) {
          local_30 = local_28;
        }
        else {
          if (local_58 == 0) {
            return 0;
          }
          for (local_38 = 6; local_38 < local_28; local_38 = local_38 + 6) {
          }
          if (local_28 < 6) {
            local_4c = local_28;
          }
          else {
            local_4c = local_38 - local_28;
          }
          if (local_4c == 0) {
            local_4c = 6;
          }
          if (local_58 < local_4c) {
            return 0;
          }
          *(undefined4 *)(param_1 + 0x2308) = 1;
          local_30 = local_4c;
          for (local_58 = local_58 - local_4c; 5 < local_58; local_58 = local_58 + -6) {
            local_30 = local_30 + 6;
          }
        }
        local_c = *(int *)(param_1 + 0x1654);
        if (local_30 != 0) {
          for (local_c = 0; local_c < *(int *)(param_1 + 0x1658); local_c = local_c + 1) {
            if ((*(int *)(*(int *)(param_1 + 0x3a4) + 0x48 + local_c * 0xfc) == 0) &&
               (*(int *)(local_c * 0xfc + *(int *)(param_1 + 0x3a4)) == 0)) {
              if (local_48 == 0) {
                if (local_64 == 0) {
                  if (local_68 == 0) {
                    if (local_60 == 0) {
                      if (local_40 == 0) {
                        if (local_3c == 0) break;
                        local_8 = 2;
                        local_10 = 4;
                        local_3c = local_3c + -1;
                      }
                      else {
                        local_8 = 1;
                        local_10 = 4;
                        local_40 = local_40 + -1;
                      }
                    }
                    else {
                      local_8 = 2;
                      local_10 = 3;
                      local_60 = local_60 + -1;
                    }
                  }
                  else {
                    local_8 = 1;
                    local_10 = 3;
                    local_68 = local_68 + -1;
                  }
                }
                else {
                  local_8 = 2;
                  local_10 = 2;
                  local_64 = local_64 + -1;
                }
              }
              else {
                local_8 = 1;
                local_10 = 2;
                local_48 = local_48 + -1;
              }
              *(int *)(local_c * 0xfc + *(int *)(param_1 + 0x3a4)) = local_8;
              *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_c * 0xfc) = local_10;
              for (local_24 = 1; local_24 < local_10; local_24 = local_24 + 1) {
                if (local_8 == 1) {
                  *(undefined4 *)(local_c * 0xfc + *(int *)(param_1 + 0x3a4) + 0x1c + local_24 * 4)
                       = 0xffffffff;
                }
                else {
                  *(undefined4 *)(local_c * 0xfc + *(int *)(param_1 + 0x3a4) + 0x1c + local_24 * 4)
                       = 0xfffffffe;
                }
                *(int *)(*(int *)(param_1 + 0x3a4) + 0xbc + local_c * 0xfc) =
                     *(int *)(*(int *)(param_1 + 0x3a4) + 0xbc + local_c * 0xfc) + 1;
              }
              local_28 = local_28 + -1;
              local_30 = local_30 + -1;
              if (local_30 == 0) break;
            }
          }
        }
        if ((bVar1) && (local_28 != 0)) {
          for (; local_c < *(int *)(param_1 + 0x1658); local_c = local_c + 1) {
            if ((*(int *)(*(int *)(param_1 + 0x3a4) + 0x48 + local_c * 0xfc) == 0) &&
               (*(int *)(local_c * 0xfc + *(int *)(param_1 + 0x3a4)) == 0)) {
              *(int *)(local_c * 0xfc + *(int *)(param_1 + 0x3a4)) = local_14;
              *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_c * 0xfc) = local_1c;
              for (local_24 = 1; local_24 < local_1c; local_24 = local_24 + 1) {
                if (local_14 == 1) {
                  *(undefined4 *)(local_c * 0xfc + *(int *)(param_1 + 0x3a4) + 0x1c + local_24 * 4)
                       = 0xffffffff;
                }
                else {
                  *(undefined4 *)(local_c * 0xfc + *(int *)(param_1 + 0x3a4) + 0x1c + local_24 * 4)
                       = 0xfffffffe;
                }
                *(int *)(*(int *)(param_1 + 0x3a4) + 0xbc + local_c * 0xfc) =
                     *(int *)(*(int *)(param_1 + 0x3a4) + 0xbc + local_c * 0xfc) + 1;
              }
            }
          }
        }
      }
    }
    else {
      for (local_c = *(int *)(param_1 + 0x1654); local_c < *(int *)(param_1 + 0x1658);
          local_c = local_c + 1) {
        if (*(int *)(local_c * 0xfc + *(int *)(param_1 + 0x3a4)) == 0) {
          *(int *)(local_c * 0xfc + *(int *)(param_1 + 0x3a4)) = local_14;
          *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_c * 0xfc) = local_1c;
          for (local_24 = 1; local_24 < local_1c; local_24 = local_24 + 1) {
            if (local_14 == 1) {
              *(undefined4 *)(local_c * 0xfc + *(int *)(param_1 + 0x3a4) + 0x1c + local_24 * 4) =
                   0xffffffff;
            }
            else {
              *(undefined4 *)(local_c * 0xfc + *(int *)(param_1 + 0x3a4) + 0x1c + local_24 * 4) =
                   0xfffffffe;
            }
            *(int *)(*(int *)(param_1 + 0x3a4) + 0xbc + local_c * 0xfc) =
                 *(int *)(*(int *)(param_1 + 0x3a4) + 0xbc + local_c * 0xfc) + 1;
          }
        }
      }
    }
  }
  return 0;
}
