/* 0040dc71 FUN_0040dc71 */

undefined4 __cdecl FUN_0040dc71(char *param_1)

{
  char *pcVar1;
  undefined4 local_8;
  
  local_8 = 0;
  pcVar1 = _strchr(param_1,0x40);
  if (pcVar1 == (char *)0x0) {
    pcVar1 = _strchr(param_1,0x23);
    if (pcVar1 == (char *)0x0) {
      pcVar1 = _strchr(param_1,0x22);
      if (pcVar1 == (char *)0x0) {
        pcVar1 = _strchr(param_1,0x5c);
        if (pcVar1 == (char *)0x0) {
          pcVar1 = _strchr(param_1,0x7b);
          if (pcVar1 == (char *)0x0) {
            pcVar1 = _strchr(param_1,0x7d);
            if (pcVar1 == (char *)0x0) {
              pcVar1 = _strchr(param_1,0x2c);
              if (pcVar1 != (char *)0x0) {
                local_8 = 0x1f002c;
              }
            }
            else {
              local_8 = 0x1f007d;
            }
          }
          else {
            local_8 = 0x1f007b;
          }
        }
        else {
          local_8 = 0x1f005c;
        }
      }
      else {
        local_8 = 0x1f0022;
      }
    }
    else {
      local_8 = 0x1f0023;
    }
  }
  else {
    local_8 = 0x1f0040;
  }
  return local_8;
}
