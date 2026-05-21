/* 00414592 FUN_00414592 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

int __thiscall FUN_00414592(void *this,uint *param_1,uint *param_2)

{
  int iVar1;
  char *pcVar2;
  uint uVar3;
  size_t sVar4;
  uint unaff_retaddr;
  char local_18c [3];
  char local_189;
  byte local_188;
  byte local_187 [355];
  uint local_24;
  uint local_20;
  int local_1c;
  byte local_18 [4];
  FILE *local_14;
  uint local_10;
  int local_c;
  uint *local_8;
  
  local_24 = DAT_00451a00 ^ unaff_retaddr;
  local_18[0] = 0x20;
  local_18[1] = 10;
  local_18[2] = 0xd;
  local_18[3] = 0;
  FUN_0043ebd0(param_2,(uint *)"Parse error. Please check equation.");
  local_14 = (FILE *)FUN_0043e6f2((char *)((int)this + 0xa90),"rt");
  if (local_14 == (FILE *)0x0) {
    local_1c = 0x2b0003;
  }
  else {
    FUN_0043f99d(local_18c,0x400,local_14);
    iVar1 = _strncmp(local_18c,".i",2);
    if (iVar1 == 0) {
      local_10 = _atol(&local_189);
      if ((int)local_10 < 0x11) {
        if ((int)local_10 < 2) {
          FUN_0043ed39((char *)param_2,(byte *)"The equation must have at least 2 input variables.")
          ;
          _fclose(local_14);
          local_1c = 1;
        }
        else {
          param_1[0x31] = local_10;
          *param_1 = 1 << ((byte)local_10 & 0x1f);
          FUN_0043f99d(local_18c,0x400,local_14);
          iVar1 = _strncmp(local_18c,".o",2);
          if (iVar1 == 0) {
            uVar3 = _atol(&local_189);
            param_1[0x32] = uVar3;
            if (param_1[0x32] < 0x11) {
              FUN_0043f99d(local_18c,0x164,local_14);
              iVar1 = _strncmp(local_18c,".ilb",4);
              if (iVar1 == 0) {
                local_8 = (uint *)FUN_0044012f(local_187,local_18);
                sVar4 = _strlen((char *)local_8);
                if (sVar4 < 9) {
                  FUN_0043ebd0(param_1 + 0x58,local_8);
                  for (local_c = 1; local_c < (int)local_10; local_c = local_c + 1) {
                    local_8 = (uint *)FUN_0044012f((byte *)0x0,local_18);
                    sVar4 = _strlen((char *)local_8);
                    if (8 < sVar4) {
                      FUN_0043ed39((char *)param_2,
                                   (byte *)"Variable names are limited to %d characters.");
                      _fclose(local_14);
                      return 1;
                    }
                    FUN_0043ebd0((uint *)((int)param_1 + local_c * 9 + 0x160),local_8);
                  }
                  FUN_0043f99d(local_18c,0x164,local_14);
                  iVar1 = _strncmp(local_18c,".ob",3);
                  if (iVar1 == 0) {
                    local_8 = (uint *)FUN_0044012f(&local_188,local_18);
                    sVar4 = _strlen((char *)local_8);
                    if (sVar4 < 9) {
                      FUN_0043ebd0(param_1 + 0x34,local_8);
                      for (local_c = 1; local_c < (int)param_1[0x32]; local_c = local_c + 1) {
                        local_8 = (uint *)FUN_0044012f((byte *)0x0,local_18);
                        sVar4 = _strlen((char *)local_8);
                        if (8 < sVar4) {
                          FUN_0043ed39((char *)param_2,
                                       (byte *)"Variable names are limited to %d characters.");
                          _fclose(local_14);
                          return 1;
                        }
                        FUN_0043ebd0((uint *)((int)param_1 + local_c * 9 + 0xd0),local_8);
                      }
                      FUN_0043f99d(local_18c,0x400,local_14);
                      iVar1 = _strncmp(local_18c,".p",2);
                      if (iVar1 == 0) {
                        local_20 = _atol(&local_189);
                        param_1[0x7d] = local_20;
                        local_1c = FUN_00421c38(param_1,local_20);
                        if (local_1c == 0) {
                          param_1[0x8f] = 0;
                          local_c = 0;
                          local_1c = 0;
                          while ((local_14->_flag & 0x10U) == 0) {
                            FUN_0043f99d(local_18c,0x164,local_14);
                            iVar1 = __strnicmp(local_18c,".e",2);
                            if (iVar1 == 0) break;
                            FUN_00421d2a(param_1,local_c,(int)local_18c);
                            local_c = local_c + 1;
                          }
                          _fclose(local_14);
                          FUN_00421eb4(param_1);
                          local_1c = 0;
                        }
                        else {
                          _fclose(local_14);
                        }
                      }
                      else {
                        _fclose(local_14);
                        local_1c = 1;
                      }
                    }
                    else {
                      FUN_0043ed39((char *)param_2,
                                   (byte *)"Variable names are limited to %d characters.");
                      _fclose(local_14);
                      local_1c = 1;
                    }
                  }
                  else {
                    _fclose(local_14);
                    local_1c = 1;
                  }
                }
                else {
                  FUN_0043ed39((char *)param_2,
                               (byte *)"Variable names are limited to %d characters.");
                  _fclose(local_14);
                  local_1c = 1;
                }
              }
              else {
                _fclose(local_14);
                local_1c = 1;
              }
            }
            else {
              FUN_0043ed39((char *)param_2,(byte *)"Too many outputs. The limit is %d.");
              _fclose(local_14);
              local_1c = 1;
            }
          }
          else {
            _fclose(local_14);
            local_1c = 1;
          }
        }
      }
      else {
        FUN_0043ed39((char *)param_2,(byte *)"Too many input variables. The limit is %d.");
        _fclose(local_14);
        local_1c = 1;
      }
    }
    else {
      _fclose(local_14);
      pcVar2 = _strstr(local_18c,"contains a cycle");
      if (pcVar2 == (char *)0x0) {
        pcVar2 = _strstr(local_18c,"bad character");
        if (pcVar2 == (char *)0x0) {
          local_1c = 1;
        }
        else {
          FUN_0043ebd0(param_2,(uint *)"Illegal character in equation.");
          local_1c = 1;
        }
      }
      else {
        local_1c = 0x180000;
      }
    }
  }
  return local_1c;
}
